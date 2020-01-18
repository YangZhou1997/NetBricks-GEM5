extern crate spmc;
use std::thread;
use std::time::Duration;
// use std::sync::mpsc;
use std::sync::mpsc::{SendError};

const STOP: usize = 0xdeadbeef;
const NUM_THREAD: usize = 1;

fn main() {

    println!("start");
    let (mut tx, rx) = spmc::channel();

    let mut handles = Vec::new();
    for n in 0..NUM_THREAD {
        let rx = rx.clone();
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

    for i in 0..(NUM_THREAD * 2) {
        let sendret = tx.send(i * 2);
        match sendret {
            Ok(()) => println!("sending succeeds: {:?}", i * 2),
            Err(SendError(t)) => println!("sending error: {:?}", t),
        }
    }

    for i in 0..NUM_THREAD {
        println!("shuting down thread {}", i);
        let sendret = tx.send(STOP);
        match sendret {
            Ok(()) => println!("sending succeeds: {:?}", STOP),
            Err(SendError(t)) => println!("sending error: {:?}", t),
        }
    }

    for handle in handles {
      handle.join().unwrap();
    }
}