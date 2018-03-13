extern crate aggregator;
extern crate futures;
extern crate tokio;

use futures::{Future, Async, Poll};
use std::thread;
use futures::sync::oneshot::Sender;
use futures::sync::oneshot::Receiver;
use std::time;
use tokio::executor::current_thread;
use aggregator::FutureSender;
use futures::Sink;

fn main() {

    let (tx, mut future_sender) = FutureSender::new();
    let mut receiver = future_sender.get_receiver();
    let mut receiver_2 = future_sender.get_receiver();


    let _ = thread::spawn(move || {
        thread::sleep(time::Duration::from_secs(1));
        println!("set");
        tx.send(10);
    });

    current_thread::run(|_| {
        current_thread::spawn(receiver);
        current_thread::spawn(future_sender);
        current_thread::spawn(receiver_2);
    });
}
