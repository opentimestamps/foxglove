extern crate futures;
extern crate tokio;

use futures::{Future, Async, Poll};
use std::thread;
use futures::sync::oneshot::Sender;
use futures::sync::oneshot::Receiver;
use std::time;
use std::time::Duration;
use tokio::executor::current_thread;

#[derive(Debug)]
pub struct FutureSender {
    rx : Receiver<u32>,
    observers: Vec<Sender<u32>>,
}

impl FutureSender {
    pub fn new() -> (Sender<u32>, FutureSender) {
        let (tx,rx) = futures::oneshot();
        (tx, FutureSender {
            rx : rx,
            observers: vec!(),
        })
    }

    pub fn get_receiver(&mut self) -> FutureReceiver {
        let (tx,rx) = futures::oneshot();
        self.observers.push(tx);
        FutureReceiver::new(rx)
    }

}

impl Future for FutureSender {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {

        match self.rx.poll().unwrap() {
            Async::Ready(v) => {
                println!("FutureSender Async::Ready({:?})",v);
                while let Some(el) = self.observers.pop() {
                    el.send(v);
                }
                Ok(Async::Ready(()))
            },
            Async::NotReady => {
                println!("FutureSender Async::NotReady");
                Ok(Async::NotReady)
            },
        }
    }
}
#[derive(Debug)]
pub struct FutureReceiver {
    rx: Receiver<u32>,
}

impl FutureReceiver {
    fn new(rx : Receiver<u32>) -> FutureReceiver {
        FutureReceiver {
            rx,
        }
    }
}

impl Future for FutureReceiver {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {

        match self.rx.poll().unwrap() {
            Async::Ready(v) => {
                println!("FutureReceiver Async::Ready({:?})",v);
                Ok(Async::Ready(()))
            },
            Async::NotReady => {
                println!("FutureReceiver Async::NotReady");
                Ok(Async::NotReady)
            },
        }
    }
}


pub trait Millis {
    fn as_millis(&self) -> f64;
}
impl Millis for Duration {
    fn as_millis(&self) -> f64 {
        self.as_secs() as f64 * 1000.0 +
            self.subsec_nanos() as f64 / 1000000.0
    }
}