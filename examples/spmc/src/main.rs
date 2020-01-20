extern crate spmc;
use std::thread;
use std::time::Duration;
// use std::sync::mpsc;
use std::sync::mpsc::{SendError};
use std::sync::mpsc;

const STOP: usize = 0xdeadbeef;
const NUM_THREAD: usize = 16;

fn facci(n: u32) -> u32 {
    // println!("{}", n);
    if n <= 1 {
        return n;
    }else{
        return facci(n-1) + facci(n-2);
    }
}


fn main() {

    println!("start");
    let (mut tx, rx) = spmc::channel();
    let (mut tx_r, rx_r) = mpsc::channel();

    let mut handles = Vec::new();
    for n in 0..NUM_THREAD {
        let rx = rx.clone();
        let tx_r = tx_r.clone();
        handles.push(thread::spawn(move || {
            loop {
                let msg = rx.recv().unwrap();
                println!("worker {} recvd: {}", n, msg);

                tx_r.send(msg).unwrap();

                // if msg == STOP {
                //     break;
                // }
                // println!("{}", facci(12));
                // do some processing. 
                // thread::sleep(Duration::from_secs(1));
            }
            // thread::sleep(Duration::from_secs(1));
            println!("thread leaves!");
        }));
    }
    for i in 0..(NUM_THREAD * 2) {
        tx.send(i * 2).unwrap();
        println!("sending succeeds: {:?}", i * 2);
        
        let r_msg = rx_r.recv().unwrap();
        println!("r_msg: {}", r_msg);

        // let sendret = tx.send(i * 2);
        // match sendret {
        //     Ok(()) => println!("sending succeeds: {:?}", i * 2),
        //     Err(SendError(t)) => println!("sending error: {:?}", t),
        // }
    }

    println!("{}", facci(12));

    // println!("shuting down thread");
    // for i in 0..NUM_THREAD {
    //     let sendret = tx.send(STOP);
    //     match sendret {
    //         Ok(()) => println!("sending succeeds: {:?}", STOP),
    //         Err(SendError(t)) => println!("sending error: {:?}", t),
    //     }
    // }

    // for handle in handles {
    //   handle.join().unwrap();
    // }
}