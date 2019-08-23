extern crate mylib;

use mylib::common::Result;
use mylib::config::load_config;
use mylib::interface::{PacketRx, PacketTx};
use mylib::operators::{Batch, ReceiveBatch};
use mylib::packets::{Ethernet, Packet, RawPacket};
use mylib::scheduler::Scheduler;
use mylib::scheduler::{initialize_system, PKT_NUM};
use std::fmt::Display;
// use std::io::stdout;
// use std::io::Write;
use std::sync::Arc;

// This "ports" is essentially "queues"
fn install<T, S>(ports: Vec<T>, sched: &mut S)
where
    T: PacketRx + PacketTx + Display + Clone + 'static,
    S: Scheduler + Sized,
{
    for port in &ports {
        println!("Receiving port {}", port);
    }

    let pipelines: Vec<_> = ports
        .iter()
        .map(|port| {
            ReceiveBatch::new(port.clone())
                .map(macswap)
                .sendall(port.clone())
        })
        .collect();

    println!("Running {} pipelines", pipelines.len());
    for pipeline in pipelines {
        sched.add_task(pipeline).unwrap();
    }
}

fn macswap(packet: RawPacket) -> Result<Ethernet> {
    assert!(packet.refcnt() == 1);
    // println!("macswap");
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    Ok(ethernet)
}

fn main() -> Result<()> {
    println!("begin");
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
