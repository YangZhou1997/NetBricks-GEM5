// use super::super::native_include as ldpdk;
// use self::ldpdk::*;
use super::super::packets::{EthernetHeader, MacAddr};
use super::super::packets::ip::v4::Ipv4Header;
use super::super::packets::ip::ProtocolNumbers;
use super::super::packets::TcpHeader;
use std::net::Ipv4Addr;
use zipf::ZipfDistribution;
use rand;
use rand::rngs::ThreadRng;
use std::cell::RefCell;
use std::sync::Arc;
use std::hash::{BuildHasherDefault, BuildHasher, Hash, Hasher};
use fxhash::FxHasher;
use rand::Rng;
use rand::RngCore;
use rand::rngs::OsRng;

#[derive(Clone)]
struct SuperBox { my_box: Box<[u8]> }

impl Drop for SuperBox {
    fn drop(&mut self) {
        unsafe {
            // println!("SuperBox freed");
        }
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
    fn drop(&mut self) {
        unsafe {
            // println!("rte_mbuf freed");
        }
    }
}
thread_local! {
    pub static ZIPF_GEN: RefCell<ZipfDistribution> = {
        let n = 3 * 1024 * 1024;
        let us = ZipfDistribution::new(n, 1.1).unwrap();
        RefCell::new(us)
    };
}

thread_local! {
    pub static RNG: RefCell<ThreadRng> = {
        let mut rng = rand::thread_rng();
        RefCell::new(rng)
    };
}

thread_local! {
    pub static RNG_STR: RefCell<OsRng> = {
        let mut r = OsRng::new().unwrap();
        RefCell::new(r)
    };
}

impl MBuf {
    #[inline]
    fn get_zipf_index() -> u64 {
        RNG.with(|rng| {
            let mut rng_mut = *rng.borrow_mut();
            ZIPF_GEN.with(|us| {
                use rand::distributions::Distribution;
                us.borrow().sample(&mut rng_mut) as u64
            })
        })
    }
    #[inline]
    fn get_zipf_five_tuples() -> (u32, u32, u16, u16) {
        let index = MBuf::get_zipf_index() as u32;
        // println!("{}", index);
 
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

    // Synthetic packet generator
    #[inline]
    pub fn new(pkt_len: u32) -> MBuf {
        // pkt_len is the length of the whole ethernet packet. 
        assert!(pkt_len <= (MAX_MBUF_SIZE as u32));
        let mut temp_vec: Vec<u8> = vec![0; pkt_len as usize];
        RNG_STR.with(|r| {
            (*r.borrow_mut()).fill_bytes(&mut temp_vec.as_mut_slice()[54..]);
        });
        
        let mut boxed: SuperBox = SuperBox{ my_box: temp_vec.into_boxed_slice(), }; // Box<[u8]> is just like &[u8];
        let address = &mut boxed.my_box[0] as *mut u8;

        let (srcip, dstip, srcport, dstport) = MBuf::get_zipf_five_tuples();
        unsafe{
            let eth_hdr: *mut EthernetHeader = address.offset(0) as *mut EthernetHeader;
            let ip_hdr: *mut Ipv4Header = address.offset(14) as *mut Ipv4Header;
            let tcp_hdr: *mut TcpHeader = address.offset(14 + 20) as *mut TcpHeader;
            (*eth_hdr).init(MacAddr::new(1, 2, 3, 4, 5, 6), MacAddr::new(0xa, 0xb, 0xc, 0xd, 0xf, 0xf));
            (*ip_hdr).init(MBuf::get_ipv4addr_from_u32(srcip), MBuf::get_ipv4addr_from_u32(dstip), ProtocolNumbers::Tcp, (pkt_len - 14) as u16);
            (*tcp_hdr).init(srcport, dstport);
        }
        
        // let buf_addr point to the start of the ethernet packet. 
        // let data_off be 0.
        MBuf{
            buf_addr: address,
            boxed,
            data_off: 0,
            pkt_len: pkt_len,
            data_len: pkt_len as u16,
            buf_len: pkt_len as u16,
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
