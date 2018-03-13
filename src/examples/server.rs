extern crate aggregator;
extern crate hyper;
extern crate futures;
extern crate rand;
extern crate tokio_core;

use futures::future::Future;
use futures::future;
use hyper::header::ContentLength;
use hyper::server::{Http, Request, Response, Service};
use std::thread;
use rand::Rng;
use tokio_core::reactor::{Remote, Handle};
use futures::sync::mpsc::UnboundedSender;
use futures::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Instant, Duration };
use aggregator::FutureSender;
use futures::Sink;
use futures::Stream;
const PHRASE: &'static str = "Hello, World!";
const TIME_SLICE_MILLIS: u64 = 100;
use tokio_core::reactor::Core;
use futures::sync::mpsc::unbounded;


struct AggregatorServerData {
    tx_digest : UnboundedSender<u32>,
    handle: Handle,
}

impl AggregatorServerData {
    fn new(tx_digest : UnboundedSender<u32>, handle : Handle) -> AggregatorServerData {
        AggregatorServerData {
            tx_digest,
            handle,
        }
    }
}

fn main() {
    let (tx, rx) = unbounded();

    let addr = "127.0.0.1:3000".parse().unwrap();
    let mut core = Core::new().unwrap();

    let handle = core.handle();
    let client_handle = core.handle();

    let server_tx_digest = tx.clone();
    let server = Http::new().serve_addr_handle(&addr, &handle,move || Ok(
        AggregatorServerData::new(server_tx_digest.clone(), client_handle.clone())
    )).unwrap();

    let h2 = handle.clone();
    handle.spawn(server.for_each(move |conn| {
        h2.spawn(conn.map(|_| ()).map_err(|err| println!("serve error: {:?}", err)));
        Ok(())
    }).map_err(|_| ()));

    let remote = handle.remote();

    let receiver = rx.for_each(move |res| {
        println!("receive {:?}",res);
        Ok(())
    }).map_err(|_| {
        println!("err");
        ()
    });

    thread::spawn(move || {
        for i in 0..2 {
            println!("Send {}", i);
            tx.unbounded_send(i);
        }
    });
    core.run(receiver).unwrap();
}



impl Service for AggregatorServerData {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, _req: Request) -> Self::Future {

        let tx=self.tx_digest.clone();
        self.handle.spawn(future::lazy(move|| {
            let mut rng = rand::thread_rng();
            tx.unbounded_send(rng.gen::<u32>());
            Ok(())
        }));


        /// returning future must be dependant to an intemediate one receiving the final result
        /// as example:
        /*let web_res_future = client.request(req);
        Box::new(web_res_future.map(|web_res| {
            let body: ResponseStream = Box::new(web_res.body().map(|b| {
                Chunk::from(format!("before: '{:?}'<br>after: '{:?}'",
                                    std::str::from_utf8(LOWERCASE).unwrap(),
                                    std::str::from_utf8(&b).unwrap()))
            }));
            Response::new().with_body(body)*/
        Box::new(futures::future::ok(
            Response::new()
                .with_header(ContentLength(PHRASE.len() as u64))
                .with_body(PHRASE)
        ))
    }
}
/*
fn aggregator_start(rx_digest : mpsc::Receiver<u32>, remote : &Handle ) {
    let time_slice_millis: Duration = Duration::from_millis(TIME_SLICE_MILLIS);

    let mut i = 0;

    let (mut last_setter, mut last_future) = FutureSender::new();
    let mut last_time = Instant::now();
    loop {
        if last_time.elapsed() >= time_slice_millis {
            last_setter.clone().send(i);
            let (mut last_setter, mut last_future) = FutureSender::new();
        }

        match rx_digest.try_recv() {
            Ok(result) => {
                last_time = Instant::now();
                let receiver = last_future.get_receiver();
                println!("{:?}", receiver);
            },
            Err(_) => {
                thread::sleep(Duration::from_millis(1));
            }
        }
        i = i+1;
    }
}*/