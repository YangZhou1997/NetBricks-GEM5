// use super::super::native_include as ldpdk;
// use self::ldpdk::*;
use super::super::packets::{EthernetHeader, MacAddr};
use super::super::packets::ip::v4::Ipv4Header;
use super::super::packets::ip::ProtocolNumbers;
use super::super::packets::TcpHeader;
use super::super::scheduler::PKT_NUM;
use std::net::Ipv4Addr;
// use std::cell::RefCell;
use std::sync::RwLock;
use std::sync::Arc;
use std::hash::{BuildHasherDefault, BuildHasher, Hash, Hasher};
use fxhash::FxHasher;
use zipfgen::ZipfDistribution;
use std::fs::File;
use std::io::Write;
use std::io::Read;
use std::io::stdout;

#[derive(Clone)]
#[derive(Debug)]
#[derive(Copy)]
pub struct MBuf_T {
    pub buf_addr: u64,
    pub data_off: u16, 
    pub pkt_len: u32,
    pub data_len: u16,
    pub buf_len: u16,
}

impl MBuf_T {
    pub unsafe fn to_mbuf_t(buf: *mut MBuf) -> MBuf_T {
        // let buf_addr = std::mem::transmute::<*mut u8, u32>((*buf).buf_addr) as u64;
        let buf_addr = std::mem::transmute::<*mut u8, u64>((*buf).buf_addr);
        
        MBuf_T {
            buf_addr: buf_addr,
            data_off: (*buf).data_off,
            pkt_len: (*buf).pkt_len,
            data_len: (*buf).data_len,
            buf_len: (*buf).buf_len,
        }
    }
    pub unsafe fn to_mbuf(buf_t: MBuf_T) -> MBuf {
        // let buf_addr = std::mem::transmute::<u32, *mut u8>(buf_t.buf_addr as u32);
        let buf_addr = std::mem::transmute::<u64, *mut u8>(buf_t.buf_addr);

        
        MBuf {
            buf_addr: buf_addr,
            data_off: buf_t.data_off,
            pkt_len: buf_t.pkt_len,
            data_len: buf_t.data_len,
            buf_len: buf_t.buf_len,
        }
    }
}

#[derive(Clone)]
pub struct rte_mbuf {
    pub buf_addr: *mut u8,
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
// void __cdecl srand (unsigned int seed)
// {
//     #ifdef _MT
//         _getptd()->_holdrand = (unsigned long)seed;
//     #else /* _MT */
//         holdrand = (long)seed;
//     #endif /* _MT */
// }

// int __cdecl rand (void)
// {
//    #ifdef _MT
//     _ptiddata ptd = _getptd();
//     return( ((ptd->_holdrand = ptd->_holdrand * 214013L + 2531011L) >> 16) &
//     0x7fff );
//    #else /* _MT */
//     return(((holdrand = holdrand * 214013L + 2531011L) >> 16) & 0x7fff);
//    #endif /* _MT */
// }

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
        // self.holdrand = self.holdrand.wrapping_mul(214013).wrapping_add(2531011);
        // let new_rand = ((self.holdrand) >> 16) & 0x7fff;
        
        hasher.write_u64(self.holdrand);
        let new_rand = hasher.finish() as u64;       
        self.holdrand = new_rand;
        new_rand
    }
}


// Simulate 
pub struct rawpkt {
    pub len: u16,
    pub raw: Box<[u8]>, // Box<[u8]> = &[u8]
}

pub struct pktgen {
    pub pkts: Box<[rawpkt]>,
    pub rand: myrand,
    pub zipf: ZipfDistribution,
    pub cur_index: usize,
}
impl pktgen {
    pub fn new(filename: &str) -> pktgen {
        println!("trying to open file {}", filename);
        stdout().flush().unwrap();
        let mut file = match File::open(filename) {
            Ok(file) => file,
            Err(why) => panic!("couldn't open {}", why),
        };

        println!("opening succeeds");
        stdout().flush().unwrap();
        let mut pkts_temp: Vec<rawpkt> = Vec::new();
        for i in 0..PKT_NUM {
            let mut buf = [0u8; 2];
            file.read(&mut buf).unwrap();
            let pkt_len: u16 = as_u16_le(&buf);
            // println!("{}", pkt_len);

            let mut temp_vec = vec![0u8; pkt_len as usize];
            file.read_exact(&mut temp_vec).unwrap();
            
            pkts_temp.push(rawpkt {
                len: pkt_len,
                raw: temp_vec.into_boxed_slice(),
            })
        }
        println!("Reading pkt trace done!");
        pktgen {
            pkts: pkts_temp.into_boxed_slice(),
            rand: myrand::new(),
            zipf: ZipfDistribution::new(PKT_NUM as usize, 1.1).unwrap(),
            cur_index: 0,
        }
    }
    pub fn next(&mut self) -> (*mut u8, u16) {
        let cur = self.cur_index;
        if cur >= PKT_LEN as usize {
            let r = self.rand.rand();
            let zipf_r = self.zipf.next(r) - 1;
            return (&mut self.pkts[zipf_r].raw[0] as *mut u8, self.pkts[zipf_r].len);
        }
        self.cur_index += 1;
        (&mut self.pkts[cur].raw[0] as *mut u8, self.pkts[cur].len)
    }
}
lazy_static! {
    static ref PKTGEN: Arc<RwLock<pktgen>> = {
        Arc::new(RwLock::new(pktgen::new("/users/yangzhou/ictf2010_1Mflow.dat")))
        // Arc::new(RwLock::new(pktgen::new("/users/yangzhou/ictf2010.dat")))
    };
}

lazy_static! {
    static ref FILE_CNT: Arc<RwLock<usize>> = {
        let cnt = 0;
        Arc::new(RwLock::new(cnt))
    };
}

fn as_u16_be(array: &[u8; 2]) -> u16 {
    ((array[0] as u16) <<  8) +
    ((array[1] as u16) <<  0)
}
fn as_u16_le(array: &[u8; 2]) -> u16 {
    ((array[0] as u16) <<  0) +
    ((array[1] as u16) <<  8)
}

impl MBuf {

    // Synthetic packet generator
    #[inline]
    pub fn new(pkt_len: u32) -> MBuf {
        // pkt_len is the length of the whole ethernet packet. 
        assert!(pkt_len <= (MAX_MBUF_SIZE as u32));

        let (address, pkt_len) = PKTGEN.write().unwrap().next();

        if cfg!(feature = "dumptrace"){
            let mut cnt = FILE_CNT.write().unwrap();
            let file_name = format!("/users/yangzhou/NetBricks-GEM5/examples/dumptrace/trace/trace_{}.txt", cnt);
            *cnt += 1;
            *cnt %= 32;

            let mut file = File::create(file_name).unwrap();
            let slice = unsafe { std::slice::from_raw_parts(address, pkt_len as usize) };
            file.write_all(slice).unwrap();
            // println!("dumptrace");
        }
        
        // let buf_addr point to the start of the ethernet packet. 
        // let data_off be 0.
        MBuf{
            buf_addr: address,
            data_off: 0,
            pkt_len: pkt_len as u32,
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
