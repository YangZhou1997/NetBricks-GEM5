extern crate crossbeam_deque as deque;
extern crate crossbeam_utils as utils;
extern crate crossbeam_queue as queue;
extern crate spmc;
extern crate colored;
extern crate fnv;
#[macro_use]
extern crate lazy_static;
extern crate netbricks;
extern crate rand;
extern crate aho_corasick;
use netbricks::common::Result;
use netbricks::config::load_config;
use netbricks::interface::{PacketRx, PacketTx};
use netbricks::operators::{Batch, ReceiveBatch};
use std::fmt::Display;
// use colored::*;
// use std::net::Ipv4Addr;
use netbricks::scheduler::Scheduler;
use netbricks::scheduler::{initialize_system, PKT_NUM};
use std::sync::RwLock;
use std::sync::Arc;

use std::thread;
use std::time::Duration;
use std::sync::mpsc;
use std::sync::mpsc::{SendError, TryRecvError};
use spmc::channel::{Sender, Receiver};

use netbricks::packets::ip::v4::Ipv4;
use netbricks::packets::{Ethernet, Packet, RawPacket, Tcp};
use std::str;
use std::io::stdout;
use std::io::Write;
use aho_corasick::AhoCorasick;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::cell::RefCell;
use netbricks::utils::DPIRULES;
use netbricks::scheduler::*;
use netbricks::operators::BATCH_SIZE;
use netbricks::native::mbuf::MBuf;
use netbricks::native::mbuf::MBuf_T;
use std::convert::TryInto;
use deque::Steal::{Empty, Success};
use deque::{Injector, Worker};
use queue::ArrayQueue;
use utils::thread::scope;

const STOP: u32 = 0xdeadbeef;
const NUM_THREAD: usize = 16;

const RULE_NUM: usize = (1 << 30);
// const RULE_NUM: usize = 32; 


/* According to my customized pktgen_zeroloss: */
// set pkt_size: 48 includes the 4B pkt_idx, 2B burst_size, and 2B identifier;
// int pkt_size = 48 + sizeof(struct ether_hdr); // 48 + 14 = 62 bytes
// const PAYLOAD_OFFSET: usize = 62; // payload offset relative to the ethernet header.

lazy_static! {
    static ref AC: Arc<AhoCorasick> = {
        let mut rules = vec![];

        for line in DPIRULES.iter() {
            rules.push(line);
        }
        if RULE_NUM < rules.len() {
            rules.truncate(RULE_NUM);
        }
        println!("dpi rules length: {}", rules.len());
        //let patterns = &["This is", "Yang", "abcedf"];
        let patterns = &rules;
        let m = AhoCorasick::new(patterns);
        println!("dpi AhoCorasick graph built up!");
        Arc::new(m)
    };
}

fn main() -> Result<()> {
    use std::env;
    let argvs: Vec<String> = env::args().collect();
    let mut pkt_num = PKT_NUM; // 1 * 1024 * 1024
    if argvs.len() == 2 {
        pkt_num = argvs[1].parse::<u64>().unwrap();
    }
    println!("pkt_num: {}", pkt_num);
    
    let payload: &[u8; 10] = &[0; 10];
    for mat in AC.find_iter(payload) {
        println!("{:?}", mat);
    }
    let q = Injector::new();
    let rq = ArrayQueue::<usize>::new(1000);

    scope(|scope| {
        for i in 0..NUM_THREAD {
            scope.spawn(|i| {
                loop {
                    let w2 = Worker::new_fifo();
                    if let Success(_) = q.steal_batch(&w2) {
                        while let Some(n) = w2.pop() {
                            unsafe{
                                let mut mbuf = MBuf_T::to_mbuf(n as MBuf_T);

                                let packet = RawPacket::from_mbuf(&mut mbuf as *mut MBuf);
                                let mut ethernet = packet.parse::<Ethernet>().unwrap();
                                ethernet.swap_addresses();
                                let v4 = ethernet.parse::<Ipv4>().unwrap();
                                let tcp = v4.parse::<Tcp<Ipv4>>().unwrap();
                                let payload: &[u8] = tcp.get_payload();

                                let mut matches = vec![];
                                
                                for mat in AC.find_iter(payload) {
                                    matches.push((mat.pattern(), mat.start(), mat.end()));
                                }
                                // println!("{:?}", matches);
                                // println!("worker {:?} recvd pktlen: {}", i, mbuf.pkt_len);
                                // stdout().flush().unwrap();

                                while rq.push(matches.len()).is_err() {}
                            }
                        }
                    }
                }
                println!("thread {:?} leaves!", i);
            });
        }

        let mut context = initialize_system().unwrap();
        let mut buffers: Vec<*mut MBuf> = Vec::<*mut MBuf>::with_capacity(BATCH_SIZE);
        let mut total_packets = 0;
        loop {
            unsafe{buffers.set_len(BATCH_SIZE);}
            match context.rx_queues[0].recv(buffers.as_mut_slice()) {
                Ok(received) => {
                    unsafe{buffers.set_len(received as usize);}
                    for i in 0..(received as usize) {
                        unsafe{
                            q.push(MBuf_T::to_mbuf_t(buffers[i]));

                            if let Ok(x) = rq.pop() {}

                            let temp_box = Box::from_raw(buffers[i]);
                            drop(temp_box);
                        }
                    }
                    total_packets += received as usize;
                },
                // the underlying DPDK method `rte_eth_rx_burst` will
                // never return an error. The error arm is unreachable
                _ => unreachable!(),
            }
            
            if total_packets % (1024 * 1024 / 16) == 0 {
                println!("dpi packets processed: {}", total_packets);
            }

            // we let nfs in gem5 run forever; 
            // if total_packets >= (pkt_num as u32).try_into().unwrap() {
            //     break;
            // }
        }
    }).unwrap();


    Ok(())
}
