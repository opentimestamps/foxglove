extern crate ots_aggregator;
extern crate futures;
extern crate tokio;
extern crate tokio_core;

use futures::sync::oneshot;
use std::thread;
use futures::Stream;
use futures::Future;
use tokio::executor::current_thread;
use tokio_core::reactor::Core;

fn main() {
    let (tx, rx) = oneshot::channel();
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let receiver = rx.map(move |res| {
        println!("Receive {:?}", res);
    });

    let sender = tx.send(2);

    core.run(receiver).unwrap();

}
