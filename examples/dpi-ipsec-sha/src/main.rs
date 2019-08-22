extern crate colored;
extern crate fnv;
#[macro_use]
extern crate lazy_static;
extern crate netbricksipsec as netbricks;
extern crate rand;
extern crate aho_corasick;
use self::dpi::*;
use netbricks::common::Result;
use netbricks::config::load_config;
use netbricks::interface::{PacketRx, PacketTx};
use netbricks::operators::{Batch, ReceiveBatch};
use netbricks::scheduler::Scheduler;
use netbricks::scheduler::{initialize_system, PKT_NUM};
use std::sync::Arc;
use std::fmt::Display;
// use colored::*;
// use std::net::Ipv4Addr;
mod dpi;

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
                .send(port.clone())
        })
        .collect();

    println!("Running {} pipelines", pipelines.len());
    for pipeline in pipelines {
        sched.add_task(pipeline).unwrap();
    }
}

fn main() -> Result<()> {
	let configuration = load_config()?;
    println!("{}", configuration);
    use std::env;
    let argvs: Vec<String> = env::args().collect();
    let mut pkt_num = PKT_NUM; // 2 * 1024 * 1024
    if argvs.len() == 2 {
        pkt_num = argvs[1].parse::<u64>().unwrap();
    }
    println!("pkt_num: {}", pkt_num);

    let mut context = initialize_system(&configuration)?;
    context.run(Arc::new(install), pkt_num); // will trap in the run() and return after finish
    Ok(())
}
