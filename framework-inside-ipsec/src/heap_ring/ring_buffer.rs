use common::*;
use failure::Fail;
use std::cmp::min;
use std::io::{Read, Write};
use std::io::Error as IOError;
use std::ptr;
use utils::PAGE_SIZE;
use std::slice;
use native::mbuf::{MBuf, PKT_LEN};
use std::sync::atomic::compiler_fence;
use std::sync::atomic::Ordering;
use std::boxed::Box;

/// Error related to the RingBuffer
#[derive(Debug, Fail)]
#[fail(display = "Bad ring size {}, must be a power of 2", _0)]
struct InvalidRingSize(usize);

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
struct SuperVec { my_vec: *mut (*mut MBuf) }

impl Drop for SuperVec {
    fn drop(&mut self) {
        unsafe {
            println!("SuperVec inside freed");
        }
    }
}

#[derive(Clone)]
pub struct SuperUsize { pub my_usize: *mut usize }

impl Drop for SuperUsize {
    fn drop(&mut self) {
        unsafe {
            println!("SuperUsize inside freed");
        }
    }
}

#[derive(Clone)]
/// A ring buffer which can be used to insert and read ordered data.
pub struct RingBuffer {
    /// boxed ring; avoid heap memory being dropped;
    boxed: SuperBox,
    /// Head, signifies where a consumer should read from.
    pub head: SuperUsize,
    /// Tail, signifies where a producer should write.
    pub tail: SuperUsize,
    /// Size of the ring buffer.
    pub size: SuperUsize,
    /// Mask used for bit-wise wrapping operations.
    pub mask: SuperUsize,
    /// A Vec that holds this RingBuffer's data.
    vec: SuperVec,
}

// once receiving ctrl+c signal, pktpuller will set size to 0xabcdefff, 
// in this way, the enclave can know it should stop itself. 

impl Drop for RingBuffer {
    fn drop(&mut self) {
        unsafe {
            println!("RingBuffer inside freed");
        }
    }
}
unsafe impl Sync for RingBuffer {}
unsafe impl Send for RingBuffer {}

#[cfg_attr(feature = "dev", allow(len_without_is_empty))]
impl RingBuffer {
    /// Create a new wrapping ring buffer. The ring buffer size is specified in bytes and must be a power of 2. 
    /// bytes is the number of bytes of RingBuffer::vec
    /// we will require additional 16 bytes to store the meta-data for this ring.
    pub unsafe fn new_in_heap(ring_size: usize) -> Result<RingBuffer>{
        if ring_size & (ring_size - 1) != 0 {
            // We need pages to be a power of 2.
            return Err(InvalidRingSize(ring_size).into());
        }

        let temp_vec: Vec<u8> = vec![0; ring_size * 8 + 16];
        let mut boxed: SuperBox = SuperBox{ my_box: temp_vec.into_boxed_slice(), }; // Box<[u8]> is just like &[u8];

        let address = &mut boxed.my_box[0] as *mut u8;
        unsafe{
            *(address as *mut usize) = 0;
            *((address as *mut usize).offset(1)) = 0;
            *((address as *mut usize).offset(2)) = ring_size;
            *((address as *mut usize).offset(3)) = ring_size - 1;
        }

        let mut rb = RingBuffer {
            boxed,
            head: SuperUsize{ my_usize: (address as *mut usize) },
            tail: SuperUsize{ my_usize: (address as *mut usize).offset(1) }, 
            size: SuperUsize{ my_usize: (address as *mut usize).offset(2) },
            mask: SuperUsize{ my_usize: (address as *mut usize).offset(3) },
            vec: SuperVec{ my_vec: (address as *mut usize).offset(4) as (*mut (*mut MBuf)) },
        };
        // Write inital 32 mbufs. 
        // let mut mbufs_vec = Vec::<*mut MBuf>::with_capacity(32);
        // unsafe{mbufs_vec.set_len(32);}
        // let mut mbufs: &mut [*mut MBuf] = mbufs_vec.as_mut_slice();
        // RingBuffer::gen_mbuf_batch(mbufs, 32);
        // rb.write_at_tail(mbufs);
        // drop(mbufs);

        Ok(rb)
    }


    #[inline]
    pub fn head(&self) -> usize{
        unsafe{(*self.head.my_usize)}
    }
    #[inline]
    pub fn set_head(&self, new_head: usize){
        unsafe{*self.head.my_usize = new_head;}
    }
    #[inline]
    pub fn wrapping_sub_head(&self, delta: usize)
    {
        self.set_head(self.head().wrapping_sub(delta));        
    }
    #[inline]
    pub fn wrapping_add_head(&self, delta: usize)
    {
        self.set_head(self.head().wrapping_add(delta));        
    }

    #[inline]
    pub fn tail(&self) -> usize{
        unsafe{(*self.tail.my_usize)}
    }
    #[inline]
    pub fn set_tail(&self, new_tail: usize){
        unsafe{*self.tail.my_usize = new_tail;}
    }
    #[inline]
    pub fn wrapping_sub_tail(&self, delta: usize)
    {
        self.set_tail(self.tail().wrapping_sub(delta));        
    }
    #[inline]
    pub fn wrapping_add_tail(&self, delta: usize)
    {
        self.set_tail(self.tail().wrapping_add(delta));
    }

    #[inline]
    pub fn size(&self) -> usize{
        unsafe{(*self.size.my_usize)}
    }
    #[inline]
    pub fn set_size(&self, new_size: usize){
        unsafe{*self.size.my_usize = new_size;}
    }

    #[inline]
    pub fn mask(&self) -> usize{
        unsafe{(*self.mask.my_usize)}
    }
    #[inline]
    pub fn set_mask(&self, new_mask: usize){
        unsafe{*self.mask.my_usize = new_mask;}
    }
    
    /// Read from the buffer, incrementing the read head. Returns bytes read.
    #[inline]
    pub fn read_from_head(&self, mbufs: &mut [*mut MBuf]) -> usize {
        let mut new_mbufs_vec = Vec::<*mut MBuf>::with_capacity(32);
        unsafe{new_mbufs_vec.set_len(32);}
        let mut new_mbufs: &mut [*mut MBuf] = new_mbufs_vec.as_mut_slice();
        RingBuffer::gen_mbuf_batch(new_mbufs, 32);
        self.write_at_tail(new_mbufs);

		let ring_size = self.size();
        let available = self.tail().wrapping_sub(self.head());
        let to_read = min(mbufs.len(), available);
        let offset = self.head() & self.mask();
        let reads = self.wrapped_read(offset, &mut mbufs[..to_read]);
        compiler_fence(Ordering::Release);
        self.wrapping_add_head(reads);

        reads
    }

    /// Write data at the end of the buffer. The amount of data written might be smaller than input.
    #[inline]
    pub fn write_at_tail(&self, mbufs: &[*mut MBuf]) -> usize {
        let available = self.size().wrapping_add(self.head()).wrapping_sub(self.tail());
        let to_write = min(mbufs.len(), available);
        let offset = self.tail() & self.mask();
        let writes = self.wrapped_write(offset, &mbufs[..to_write]);
        compiler_fence(Ordering::Release);
        self.wrapping_add_tail(writes);

        writes
    }

    #[inline]
    pub fn gen_mbuf_batch(mbufs: &mut [*mut MBuf], batch_size: usize) {
        for i in 0..batch_size {
            let mut mbuf_instance = MBuf::new(PKT_LEN - 4);
            mbufs[i] = Box::leak(Box::new(mbuf_instance)) as *mut MBuf;
        }
    }
    // How to drop a leaked reference: 
    // https://doc.rust-lang.org/std/boxed/struct.Box.html
    #[inline]
    pub fn free_mbuf_batch(mbufs: &[*mut MBuf], batch_size: usize) {
        unsafe{
            use std::ptr;
            for i in 0..batch_size {
                let temp_box = Box::from_raw(mbufs[i]);
                drop(temp_box);
            }
        }
    }

    /// Reads data from self.vec, wrapping around the end of the Vec if necessary. Returns the
    /// number of bytes written.
    fn wrapped_read(&self, offset: usize, mbufs: &mut [*mut MBuf]) -> usize {
        let mut bytes: usize = 0;
        let ring_size = self.size();
        assert!(offset < ring_size);
        assert!(mbufs.len() <= ring_size);

        let mut bytes = min(ring_size - offset, mbufs.len());
        if bytes != 0 {
            unsafe{ ptr::copy(self.vec.my_vec.offset(offset as isize), &mut mbufs[0] as (*mut (*mut MBuf)), bytes) };
        }
        if offset + mbufs.len() > ring_size {
            let remaining = mbufs.len() - bytes;
            unsafe{ ptr::copy(self.vec.my_vec, ((&mut mbufs[0]) as (*mut (*mut MBuf))).offset(bytes as isize), remaining) };
            bytes += remaining;
        }
        bytes
    }

    /// Writes data to self.vec[offset..], wrapping around the end of the Vec if necessary. Returns
    /// the number of bytes written.
    fn wrapped_write(&self, offset: usize, mbufs: &[*mut MBuf]) -> usize {
        let mut bytes: usize = 0;
        let ring_size = self.size();
        assert!(offset < ring_size);
        assert!(mbufs.len() <= ring_size);

        let mut bytes = min(ring_size - offset, mbufs.len());
        if bytes != 0 {
            unsafe{ ptr::copy(&mbufs[0] as (*const (* mut MBuf)), self.vec.my_vec.offset(offset as isize), bytes) };
        }
        if offset + mbufs.len() > ring_size {
            let remaining = mbufs.len() - bytes;
            unsafe{ ptr::copy(((&mbufs[0]) as (*const (* mut MBuf))).offset(bytes as isize), self.vec.my_vec, remaining) };
            bytes += remaining;
        }
        bytes
    }

    /// Length of the ring buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.size()
    }

    /// If the ring buffer is empty or not.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn clear(&self) {
        self.set_head(0);
        self.set_tail(0);
    }
}
