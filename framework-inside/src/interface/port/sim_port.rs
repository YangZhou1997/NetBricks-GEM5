use super::super::{PacketRx, PacketTx};
use super::PortStats;
use allocators::*;
use common::*;
use native::mbuf::{MBuf, MAX_MBUF_SIZE};
use native::{mbuf_alloc_bulk, mbuf_free_bulk};
use std::fmt;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use config::{PortConfiguration, NUM_RXD, NUM_TXD};
use operators::BATCH_SIZE;

use std::io::stdout;
use std::io::Write;

use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::thread;
use std::sync::Mutex;
use std::slice;

use heap_ring::ring_buffer::*;

#[link(name="mapping", kind="static")]
extern { fn mapping(); }

pub struct SimulatePort {
    stats_rx: Arc<CacheAligned<PortStats>>,
    stats_tx: Arc<CacheAligned<PortStats>>,
}

impl fmt::Debug for SimulatePort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Simulate port")
    }
}

#[derive(Clone)]
pub struct SimulateQueue {
    stats_rx: Arc<CacheAligned<PortStats>>,
    stats_tx: Arc<CacheAligned<PortStats>>,
    recvq_ring: RingBuffer,
    sendq_ring: RingBuffer,
}

impl fmt::Display for SimulateQueue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Simulate queue")
    }
}

impl PacketTx for SimulateQueue {
    #[inline]
    fn send(&self, pkts: &mut [*mut MBuf]) -> Result<u32> {
        let len = pkts.len();
        let update = self.stats_tx.stats.load(Ordering::Relaxed) + len as usize;
        self.stats_tx.stats.store(update, Ordering::Relaxed);

        let mut cur_sent = 0;
        // push len mbuf pointers to sendq.
        if !pkts.is_empty() {
            while cur_sent < len {
                let sent = self.sendq_ring.write_at_tail(&mut pkts[cur_sent..]);
                cur_sent += sent;
            }
        }
        // mbuf_free_bulk(pkts.as_mut_ptr(), len);
        Ok(len as u32)
    }
}

impl PacketRx for SimulateQueue {
    /// Send a batch of packets out this PortQueue. Note this method is internal to NetBricks (should not be directly
    /// called).
    #[inline]
    fn recv(&self, pkts: &mut [*mut MBuf]) -> Result<u32> {
        // pull packet from recvq;
        let recv_pkt_num_from_enclave = self.recvq_ring.read_from_head(pkts);
        let alloced = recv_pkt_num_from_enclave;
        let update = self.stats_rx.stats.load(Ordering::Relaxed) + alloced as usize;
        self.stats_rx.stats.store(update, Ordering::Relaxed);
        
		Ok(alloced as u32)
    }
}

fn fib(n: u64) -> u64{
    if n == 0{
        return 0;
    }
    else if n == 1{
        return 1;
    }
    else{
        return fib(n - 1) + fib(n - 2); 
    }
}

impl SimulatePort {
    pub fn new(port_config: &PortConfiguration) -> Result<Arc<SimulatePort>> {        
        Ok(Arc::new(SimulatePort {
            stats_rx: Arc::new(PortStats::new()),
            stats_tx: Arc::new(PortStats::new()),
        }))
    }

    pub fn new_simulate_queue(&self, _queue: i32) -> Result<CacheAligned<SimulateQueue>> {
        unsafe { mapping(); };
        Ok(CacheAligned::allocate(SimulateQueue {
            stats_rx: self.stats_rx.clone(),
            stats_tx: self.stats_tx.clone(),
            recvq_ring: unsafe{RingBuffer::new_in_heap((NUM_RXD) as usize, &format!("{}_{}", RECVQ_PREFIX, 0)).unwrap() },
            sendq_ring: unsafe{RingBuffer::new_in_heap((NUM_TXD) as usize, &format!("{}_{}", SENDQ_PREFIX, 0)).unwrap() },
        }))
    }

    /// Get stats for an RX/TX queue pair.
    pub fn stats(&self) -> (usize, usize) {
        (
            self.stats_rx.stats.load(Ordering::Relaxed),
            self.stats_tx.stats.load(Ordering::Relaxed),
        )
    }
}
