extern crate libc;
#[cfg_attr(test, macro_use)]
extern crate failure;
extern crate colored;
extern crate fnv;
#[macro_use]
extern crate lazy_static;
extern crate netbricks;
extern crate rand;
extern crate aho_corasick;
use self::dpi::*;
use netbricks::common::Result;
use netbricks::config::load_config;
use netbricks::interface::{PacketRx, PacketTx};
use netbricks::operators::{Batch, ReceiveBatch};
use std::fmt::Display;
// use colored::*;
// use std::net::Ipv4Addr;
use netbricks::scheduler::Scheduler;
use netbricks::scheduler::{initialize_system, PKT_NUM};
use std::sync::Arc;

use std::cmp::min;
use std::io::{Read, Write};
use std::io::Error as IOError;
use std::ptr;
use std::slice;
use std::sync::atomic::compiler_fence;
use std::sync::atomic::Ordering;
use std::ffi::CString;
use std::result::Result as stdRes;

use failure::Fail;
use failure::Error;
use libc::{c_void, close, ftruncate, mmap, munmap, shm_open, shm_unlink};
use std::convert::TryInto;

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
    let mut context = initialize_system()?;

unsafe{
    let size = 100 * 8 + 16;
    let name = CString::new("/test_shm_gem5").unwrap();
    let mut fd = shm_open(
        name.as_ptr(),
        libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
        0o700,
    );
    if fd == -1 {
        if let Some(e) = IOError::last_os_error().raw_os_error() {
            if e == libc::EEXIST {
                // println!("unlink previous shm");
                // shm_unlink(name.as_ptr());

                // if already exist, we just attach to it, instead of unlinking it. 
                println!("attach to previous shm");
                fd = shm_open(
                    name.as_ptr(),
                    libc::O_CREAT | libc::O_RDWR,
                    0o700,
                );
            }
        }
    };
    assert!(fd >= 0, "Could not create shared memory segment");
    let ftret = ftruncate(fd, (size as i64).try_into().unwrap());
    assert!(ftret == 0, "Could not truncate");
    let address = mmap(
        ptr::null_mut(),
        size,
        libc::PROT_READ | libc::PROT_WRITE,
        libc::MAP_POPULATE | libc::MAP_SHARED,
        fd,
        0,
    );
    if address == libc::MAP_FAILED {
        let err_string = CString::new("mmap failed").unwrap();
        libc::perror(err_string.as_ptr());
        panic!("Could not mmap shared region");
    }
    close(fd);

    let address = address as *mut u8;
    println!("{:?}", address);
}

    context.run(Arc::new(install), pkt_num); // will trap in the run() and return after finish
    Ok(())
}
