// use super::super::native_include as ldpdk;
// use self::ldpdk::*;
use super::super::packets::{EthernetHeader, MacAddr};
use super::super::packets::ip::v4::Ipv4Header;
use super::super::packets::ip::ProtocolNumbers;
use super::super::packets::TcpHeader;
use std::net::Ipv4Addr;

#[derive(Clone)]
struct SuperBox { my_box: Box<[u8]> }

impl Drop for SuperBox {
    fn drop(&mut self) {
        unsafe {
            println!("SuperBox freed");
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

impl Drop for MBuf {
    fn drop(&mut self) {
        unsafe {
            println!("rte_mbuf freed");
        }
    }
}

impl MBuf {
    // what I need to do is to 
    // let buf_addr point to the start of the ethernet packet. 
    // let data_off be 0.
    #[inline]
    pub fn new(pkt_len: u32) -> MBuf {
        // pkt_len is the length of the whole ethernet packet. 
        assert!(pkt_len <= (MAX_MBUF_SIZE as u32));
        let mut temp_vec: Vec<u8> = vec![0; pkt_len as usize];
        let mut boxed: SuperBox = SuperBox{ my_box: temp_vec.into_boxed_slice(), }; // Box<[u8]> is just like &[u8];
        let address = &mut boxed.my_box[0] as *mut u8;

        unsafe{
            let eth_hdr: *mut EthernetHeader = address.offset(0) as *mut EthernetHeader;
            let ip_hdr: *mut Ipv4Header = address.offset(14) as *mut Ipv4Header;
            let tcp_hdr: *mut TcpHeader = address.offset(14 + 20) as *mut TcpHeader;
            (*eth_hdr).init(MacAddr::new(1, 2, 3, 4, 5, 6), MacAddr::new(0xa, 0xb, 0xc, 0xd, 0xf, 0xf));
            (*ip_hdr).init(Ipv4Addr::new(127, 0, 0, 1), Ipv4Addr::new(127, 0, 0, 2), ProtocolNumbers::Tcp, (pkt_len - 14) as u16);
            (*tcp_hdr).init(0x1234, 0xabcd);
        }

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
