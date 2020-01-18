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
// use std::sync::mpsc;
use std::sync::mpsc::{SendError};
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

const STOP: usize = 0xdeadbeef;
const NUM_THREAD: usize = 1;

const RULE_NUM: usize = (1 << 30); 

lazy_static! {
    static ref TRX: Arc<(RwLock<Sender<usize>>, Receiver<usize>)> = {
        let (tx, rx) = spmc::channel();
        Arc::new((RwLock::new(tx), tx))
    };
}

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
        Arc::new(m)
    };
}

pub fn dpi(packet: RawPacket) -> Result<Tcp<Ipv4>> {
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    let v4 = ethernet.parse::<Ipv4>()?;
    let tcp = v4.parse::<Tcp<Ipv4>>()?;
    let payload: &[u8] = tcp.get_payload();

    for i in 0..(NUM_THREAD * 2) {
        let sendret = TRX[0].write().unwrap().send(i * 2);
        match sendret {
            Ok(()) => println!("sending succeeds: {:?}", i * 2),
            Err(SendError(t)) => println!("sending error: {:?}", t),
        }
    }

    let mut matches = vec![];
    
    for mat in AC.find_iter(payload) {
        matches.push((mat.pattern(), mat.start(), mat.end()));
    }
    
    // println!("{:?}", matches);
    // stdout().flush().unwrap();

    Ok(tcp)
}


fn install<T, S>(ports: Vec<T>, sched: &mut S)
where
    T: PacketRx + PacketTx + Display + Clone + 'static,
    S: Scheduler + Sized,
{
    println!("Receiving started");
    for port in &ports {
        println!("Receiving port {}", port);
    }

    let pipelines: Vec<_> = ports
        .iter()
        .map(|port| {
            ReceiveBatch::new(port.clone())
                .map(dpi)
                .sendall(port.clone())
        })
        .collect();

    println!("Running {} pipelines", pipelines.len());
    for pipeline in pipelines {
        sched.add_task(pipeline).unwrap();
    }
}

fn main() -> Result<()> {
    use std::env;
    let argvs: Vec<String> = env::args().collect();
    let mut pkt_num = PKT_NUM; // 2 * 1024 * 1024
    if argvs.len() == 2 {
        pkt_num = argvs[1].parse::<u64>().unwrap();
    }
    println!("pkt_num: {}", pkt_num);
    
    println!("spmc start");
    
    let mut handles = Vec::new();
    for n in 0..NUM_THREAD {
        let rx = TRX[0].clone();
        handles.push(thread::spawn(move || {
            loop {
                let msg = rx.recv().unwrap();
                println!("worker {} recvd: {}", n, msg);
                // do some processing. 
                // thread::sleep(Duration::from_secs(1));
                if msg == STOP {
                    break;
                }
            }
            // thread::sleep(Duration::from_secs(1));
            println!("thread leaves!");
        }));
    }

    let mut context = initialize_system()?;
    context.run(Arc::new(install), pkt_num); // will trap in the run() and return after finish

    for i in 0..NUM_THREAD {
        println!("shuting down thread {}", i);
        let sendret = TRX[1].write().unwrap().send(STOP);
        match sendret {
            Ok(()) => println!("sending succeeds: {:?}", STOP),
            Err(SendError(t)) => println!("sending error: {:?}", t),
        }
    }

    for handle in handles {
      handle.join().unwrap();
    }

    Ok(())
}
