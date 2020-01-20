#[macro_use]
extern crate lazy_static;
use std::thread;
use std::time::Duration;
use std::sync::RwLock;
use std::sync::Arc;

const STOP: usize = 0xdeadbeef;
const NUM_THREAD: usize = 1;

lazy_static! {
    static ref AC: Arc<RwLock> = {
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

                if msg == STOP {
                    break;
                }
                // println!("{}", facci(12));
                // do some processing. 
                // thread::sleep(Duration::from_secs(1));
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

    println!("shuting down thread");
    for i in 0..NUM_THREAD {
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