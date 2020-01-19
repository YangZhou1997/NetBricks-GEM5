extern crate aho_corasick;
use netbricks::common::Result;
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
use std::time::{Duration, Instant};
use std::sync::{RwLock, Arc};

const RULE_NUM: usize = (1 << 30); 

/* According to my customized pktgen_zeroloss: */
// set pkt_size: 48 includes the 4B pkt_idx, 2B burst_size, and 2B identifier;
// int pkt_size = 48 + sizeof(struct ether_hdr); // 48 + 14 = 62 bytes
// const PAYLOAD_OFFSET: usize = 62; // payload offset relative to the ethernet header.

thread_local! {
    pub static AC: RefCell<AhoCorasick> = {
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
        RefCell::new(m)
    };
}

lazy_static! {
    static ref CNT: Arc<RwLock<u128>> = {
        let cnt = 0 as u128;
        Arc::new(RwLock::new(cnt))
    };
}
lazy_static! {
    static ref ACCU_DURATION: Arc<RwLock<u128>> = {
        let accu_duration = 0 as u128;
        Arc::new(RwLock::new(accu_duration))
    };
}

pub fn dpi(packet: RawPacket) -> Result<Tcp<Ipv4>> {
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    let v4 = ethernet.parse::<Ipv4>()?;
    let tcp = v4.parse::<Tcp<Ipv4>>()?;
    let payload: &[u8] = tcp.get_payload();

    // println!("{}", payload.len());
    // stdout().flush().unwrap();
    
    // let payload_str = match str::from_utf8(&payload[..]) {
    //     Ok(v) => v,
    //     Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
    // };
    // from_utf8_unchecked

    // println!("{}", payload_str);
    // stdout().flush().unwrap();

    let mut matches = vec![];
    // let mut matches_cnt = 0;
    AC.with(|ac| {
        let start = Instant::now();
        for mat in ac.borrow().find_iter(payload) {
            // matches_cnt += 1;
            matches.push((mat.pattern(), mat.start(), mat.end()));
        }
        let duration: u128 = start.elapsed().as_micros();

        let mut accu_duration = ACCU_DURATION.write().unwrap();
        let mut cnt = CNT.write().unwrap();

        *accu_duration += duration;
        *cnt += 1;
    });
    let cnt = CNT.read().unwrap();
    let accu_duration = ACCU_DURATION.read().unwrap();
    if *cnt % (10 * 1024) == 0 {
        println!("average processing time per packet = {:?} us", (*accu_duration as f64) / (*cnt as f64));
    }

    // println!("{:?}", matches);
    // stdout().flush().unwrap();

    Ok(tcp)
}
