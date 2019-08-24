// use super::super::native_include as ldpdk;
// use self::ldpdk::*;
use super::super::packets::{EthernetHeader, MacAddr};
use super::super::packets::ip::v4::Ipv4Header;
use super::super::packets::ip::ProtocolNumbers;
use super::super::packets::TcpHeader;
use super::super::utils::ipsec::*;
use std::net::Ipv4Addr;
use std::sync::RwLock;
use std::sync::Arc;
use std::hash::{BuildHasherDefault, BuildHasher, Hash, Hasher};
use fxhash::FxHasher;
use std::io::stdout;
use std::io::Write;

#[derive(Clone)]
struct SuperBox { my_box: Box<[u8]> }

impl Drop for SuperBox {
    fn drop (self: &'_ mut Self)
    {
        // println!("Dropping boxed at {:p}", self);
    }
}

#[derive(Clone)]
pub struct rte_mbuf {
    pub buf_addr: *mut u8,
    boxed: SuperBox, 
    pub data_off: u16, 
    pub pkt_len: u32,
    pub data_len: u16,
    pub buf_len: u16,
}

pub type MBuf = rte_mbuf;

pub const MAX_MBUF_SIZE: u16 = 2048;
pub const PKT_LEN: u32 = 1024;

impl Drop for MBuf {
    fn drop (self: &'_ mut Self)
    {
        // println!("Dropping MBuf at {:p}", self);
    }
}

pub struct myrand {
    pub holdrand: u64,
}

impl myrand {
    pub fn new() -> myrand {
        let timespec = time::get_time(); 
        let mills = timespec.sec + timespec.nsec as i64 / 1000 / 1000;
        myrand {
            holdrand: mills as u64,
        }
    }
    pub fn rand(&mut self) -> u64{
        let mut hasher = FxHasher::default();
        hasher.write_u64(self.holdrand);
        let new_rand = hasher.finish() as u64;         
        self.holdrand = new_rand;
        new_rand
    }
}

lazy_static! {
    static ref RAND_GEN: Arc<RwLock<myrand>> = {
        let r = myrand::new();
        Arc::new(RwLock::new(r))
    };
}

impl MBuf {
// We borrow this code from https://answers.launchpad.net/polygraph/+faq/1478. 
// The corresponding paper is http://ldc.usb.ve/~mcuriel/Cursos/WC/spe2003.pdf.gz
// int popzipf(int n, long double skew) {
//     // popZipf(skew) = wss + 1 - floor((wss + 1) ** (x ** skew))
//     long double u = rand() / (long double) (RAND_MAX);
//     return (int) (n + 1 - floor(pow(n + 1, pow(u, skew))));
// }
    #[inline]
    fn get_zipf_index(index_range: u32, skew: f64) -> u64 {
        let r = RAND_GEN.write().unwrap().rand();
        let u: f64 = r as f64 / std::u64::MAX as f64;
        let n1: f64 = (index_range + 1) as f64 * 1.0;
        let zipf_r: u64 = (n1 - (n1.powf(u.powf(skew))).floor()) as u64;
        // println!("{}", zipf_r);
        zipf_r

    }
    #[inline]
    fn get_zipf_five_tuples() -> (u32, u32, u16, u16) {
        let index = MBuf::get_zipf_index(3 * 1024 * 1024, 1.1) as u32;
 
        let mut hasher = FxHasher::default();
        hasher.write_u32(index);
        let srcip = hasher.finish() as u32;
        hasher.write_u32(srcip);
        let dstip = hasher.finish() as u32; 
        hasher.write_u16(index as u16);
        let srcport = hasher.finish() as u16;
        hasher.write_u16(srcport);
        let dstport = hasher.finish() as u16;

        (srcip, dstip, srcport, dstport)
    }

    #[inline]
    fn get_ipv4addr_from_u32(ip: u32) -> Ipv4Addr {
        Ipv4Addr::new(((ip >> 24) & 0xFF) as u8, ((ip >> 16) & 0xFF) as u8, ((ip >> 8) & 0xFF) as u8, (ip & 0xFF) as u8)
    }

    #[inline]
    fn get_rand_str(slice: &mut [u8]) {
        let payload_len = slice.len();
        let mut start = 0;
        for i in 0..(payload_len/64 + 1) {
            let r = RAND_GEN.write().unwrap().rand();        
            slice[start..std::cmp::min(start + 8, payload_len)].copy_from_slice(&r.to_be_bytes());
            start += 8;
        }
    }

    // Synthetic packet generator
    #[inline]
    pub fn new(pkt_len: u32) -> MBuf {
        // pkt_len is the length of the whole original ethernet packet. 
        assert!(pkt_len <= (MAX_MBUF_SIZE as u32));

        // we do not need ethernet header for tunnelled packets.
        let mut temp_vec: Vec<u8> = vec![0; (pkt_len - 14) as usize];
        MBuf::get_rand_str(&mut temp_vec.as_mut_slice()[40..]);
        
        let address = &mut (temp_vec.as_mut_slice()[0]) as *mut u8;
        let (srcip, dstip, srcport, dstport) = MBuf::get_zipf_five_tuples();
        unsafe{
            let ip_hdr: *mut Ipv4Header = address.offset(0) as *mut Ipv4Header;
            let tcp_hdr: *mut TcpHeader = address.offset(20) as *mut TcpHeader;
            (*ip_hdr).init(MBuf::get_ipv4addr_from_u32(srcip), MBuf::get_ipv4addr_from_u32(dstip), ProtocolNumbers::Tcp, (pkt_len - 14) as u16);
            (*tcp_hdr).init(srcport, dstport);
        }
        
        let new_pkt_len = pkt_len as usize + 20 + ESP_HEADER_LENGTH + AES_GCM_IV_LENGTH + ICV_LEN_GCM128;
        let mut temp_vec_ipsec: Vec<u8> = vec![0; new_pkt_len as usize];
        let mut boxed_ipsec: SuperBox = SuperBox{ my_box: temp_vec_ipsec.into_boxed_slice(), }; // Box<[u8]> is just like &[u8];
        let address_ipsec = &mut boxed_ipsec.my_box[0] as *mut u8;

        let original_pkt = unsafe { std::slice::from_raw_parts(address, (pkt_len - 14) as usize) };
        let encrypted_pkt = unsafe { std::slice::from_raw_parts_mut(address_ipsec, new_pkt_len as usize) };
        let esp_hdr_raw: Vec<u8> = vec![0x1, 0x2, 0x3, 0x4, 0x5, 0x6, 0x7, 0x8];

        let encrypted_pkt_len = aes_gcm128_encrypt_openssl(original_pkt, esp_hdr_raw.as_slice(), &mut encrypted_pkt[34..]).unwrap();

        unsafe{
            let eth_hdr: *mut EthernetHeader = address_ipsec.offset(0) as *mut EthernetHeader;
            let ip_hdr: *mut Ipv4Header = address_ipsec.offset(14) as *mut Ipv4Header;
            (*eth_hdr).init(MacAddr::new(1, 2, 3, 4, 5, 6), MacAddr::new(0xa, 0xb, 0xc, 0xd, 0xf, 0xf));
            (*ip_hdr).init(MBuf::get_ipv4addr_from_u32(srcip), MBuf::get_ipv4addr_from_u32(dstip), ProtocolNumbers::Tcp, (new_pkt_len - 14) as u16);
        }
        
        
        // let buf_addr point to the start of the ethernet packet. 
        // let data_off be 0.
        MBuf{
            buf_addr: address_ipsec,
            boxed: boxed_ipsec,
            data_off: 0,
            pkt_len: new_pkt_len as u32,
            data_len: new_pkt_len as u16,
            buf_len: new_pkt_len as u16,
        }
    }

    #[inline]
    pub fn data_address(&self, offset: usize) -> *mut u8 {
        unsafe { (self.buf_addr as *mut u8).offset(self.data_off as isize + offset as isize) }
    }

    /// Returns the total allocated size of this mbuf segment.
    /// This is a constant.
    #[inline]
    pub fn buf_len(&self) -> usize {
        self.buf_len as usize
    }

    /// Returns the length of data in this mbuf segment.
    #[inline]
    pub fn data_len(&self) -> usize {
        self.data_len as usize
    }

    /// Returns the size of the packet (across multiple mbuf segment).
    #[inline]
    pub fn pkt_len(&self) -> usize {
        self.pkt_len as usize
    }

    #[inline]
    fn pkt_headroom(&self) -> usize {
        self.data_off as usize
    }

    #[inline]
    fn pkt_tailroom(&self) -> usize {
        self.buf_len() - self.data_off as usize - self.data_len()
    }

    /// Add data to the beginning of the packet. This might fail (i.e., return 0) when no more headroom is left.
    #[inline]
    pub fn add_data_beginning(&mut self, len: usize) -> usize {
        // If only we could add a likely here.
        if len > self.pkt_headroom() {
            0
        } else {
            self.data_off -= len as u16;
            self.data_len += len as u16;
            self.pkt_len += len as u32;
            len
        }
    }

    /// Add data to the end of a packet buffer. This might fail (i.e., return 0) when no more tailroom is left. We do
    /// not currently deal with packet with multiple segments.
    #[inline]
    pub fn add_data_end(&mut self, len: usize) -> usize {
        if len > self.pkt_tailroom() {
            0
        } else {
            self.data_len += len as u16;
            self.pkt_len += len as u32;
            len
        }
    }

    #[inline]
    pub fn remove_data_beginning(&mut self, len: usize) -> usize {
        if len > self.data_len() {
            0
        } else {
            self.data_off += len as u16;
            self.data_len -= len as u16;
            self.pkt_len -= len as u32;
            len
        }
    }

    #[inline]
    pub fn remove_data_end(&mut self, len: usize) -> usize {
        if len > self.data_len() {
            0
        } else {
            self.data_len -= len as u16;
            self.pkt_len -= len as u32;
            len
        }
    }

    #[inline]
    pub fn refcnt(&self) -> u16 {
        1 as u16
        // unsafe { self.__bindgen_anon_1.refcnt }
    }

    #[inline]
    pub fn reference(&mut self) {
        // unsafe {
        //     self.__bindgen_anon_1.refcnt += 1;
        // }
    }
}
