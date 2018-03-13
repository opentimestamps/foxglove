extern crate aggregator;
extern crate futures;
extern crate tokio;
extern crate tokio_core;

use futures::sync::mpsc::unbounded;
use std::thread;
use futures::Stream;
use futures::Future;
use tokio::executor::current_thread;
use tokio_core::reactor::Core;

fn main() {
    let (tx, rx) = unbounded();
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let receiver = rx.for_each(move |res| {
        println!("Receive {:?}", res);
        Ok(())
        })
        .map_err(|e| panic!("err={:?}", e))
        .map(move |v| v);

    thread::spawn(move || {
        for i in 0..10000 {
            println!("Send {}", i);
            tx.unbounded_send(i).unwrap();
        }
    });

    core.run(receiver).unwrap();

}
