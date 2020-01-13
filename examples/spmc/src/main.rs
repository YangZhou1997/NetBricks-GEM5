extern crate spmc;
use std::thread;
use std::time::Duration;
// use std::sync::mpsc;

fn main() {

    println!("start");
    let (mut tx, rx) = spmc::channel();

    let mut handles = Vec::new();
    for n in 0..5 {
        let rx = rx.clone();
        handles.push(thread::spawn(move || {
            let msg = rx.recv().unwrap();
            println!("worker {} recvd: {}", n, msg);
            thread::sleep(Duration::from_secs(1));
        }));
    }

    for i in 0..6 {
        tx.send(i * 2).unwrap();
    }

    for handle in handles {
      handle.join().unwrap();
    }
}