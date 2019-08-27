extern crate fnv;
#[macro_use]
extern crate lazy_static;
extern crate netbricks;
use fnv::FnvHasher;
use netbricks::common::Result;
use netbricks::config::load_config;
use netbricks::interface::{PacketRx, PacketTx};
use netbricks::operators::{Batch, ReceiveBatch};
use netbricks::packets::ip::v4::Ipv4;
use netbricks::packets::ip::Flow;
use netbricks::packets::{Ethernet, Packet, RawPacket, Tcp};
use netbricks::scheduler::{initialize_system, PKT_NUM};
use netbricks::scheduler::Scheduler;
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::BuildHasherDefault;
use netbricks::utils::ipsec::*;
use std::io::stdout;
use std::io::Write;
use std::cell::RefCell;
use std::sync::Arc;


type FnvHash = BuildHasherDefault<FnvHasher>;

thread_local! {
    pub static FLOW_MAP: RefCell<HashMap<Flow, u64, FnvHash>> = {
        let m = HashMap::with_hasher(Default::default());
        RefCell::new(m)
    };
}

fn install<T, S>(ports: Vec<T>, sched: &mut S)
where
    T: PacketRx + PacketTx + Display + Clone + 'static,
    S: Scheduler + Sized,
{
    println!("Receiving started");

    let pipelines: Vec<_> = ports
        .iter()
        .map(move |port| {
            ReceiveBatch::new(port.clone())
                .map(|p| monitoring(p))
                .sendall(port.clone())
        })
        .collect();

    println!("Running {} pipelines", pipelines.len());
    for pipeline in pipelines {
        sched.add_task(pipeline).unwrap();
    }
}

fn monitoring(packet: RawPacket) -> Result<Ipv4> {
    // print!("-4");stdout().flush();
    let mut ethernet = packet.parse::<Ethernet>()?;
    ethernet.swap_addresses();
    let v4 = ethernet.parse::<Ipv4>()?;
    let payload: &mut [u8] = v4.get_payload_mut(); // payload.len()

    let esp_hdr: &mut [u8] = &mut [0u8; 8];
    esp_hdr.copy_from_slice(&payload[0..ESP_HEADER_LENGTH]);

    let decrypted_pkt: &mut [u8] = &mut [0u8; 2000];
    // let decrypted_pkt_len = aes_cbc_sha256_decrypt(payload, decrypted_pkt, false).unwrap();
    // let decrypted_pkt_len = aes_gcm128_decrypt_openssl(payload, decrypted_pkt, false).unwrap();
    let decrypted_pkt_len = aes_gcm128_decrypt_openssl(payload, decrypted_pkt, false).unwrap();

    let flow = get_flow(decrypted_pkt);
    FLOW_MAP.with(|flow_map| {
        // println!("{}", flow);stdout().flush().unwrap();
        *((*flow_map.borrow_mut()).entry(flow).or_insert(0)) += 1;
    });

    // let encrypted_pkt_len = aes_cbc_sha256_encrypt(&decrypted_pkt[..(decrypted_pkt_len - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH)], &(*esp_hdr), payload).unwrap();
    // let encrypted_pkt_len = aes_gcm128_encrypt_openssl(&decrypted_pkt[..(decrypted_pkt_len - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH)], &(*esp_hdr), payload).unwrap();
    let encrypted_pkt_len = aes_gcm128_encrypt_openssl(&decrypted_pkt[..(decrypted_pkt_len - ESP_HEADER_LENGTH - AES_CBC_IV_LENGTH)], &(*esp_hdr), payload).unwrap();
    
    Ok(v4)
}

fn main() -> Result<()> {
    use std::env;
    let argvs: Vec<String> = env::args().collect();
    let mut pkt_num = PKT_NUM; // 2 * 1024 * 1024
    if argvs.len() == 2 {
        pkt_num = argvs[1].parse::<u64>().unwrap();
    }
    println!("pkt_num: {}", pkt_num);

    let mut context = initialize_system()?;
    context.run(Arc::new(install), pkt_num); // will trap in the run() and return after finish
    Ok(())
}
