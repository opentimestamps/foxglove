extern crate aggregator;
extern crate hyper;
extern crate futures;
extern crate rand;
extern crate tokio_core;
extern crate crypto;

use std::thread;
use std::sync::{mpsc, Arc};
use rand::Rng;
use futures::Stream;
use futures::future::Future;
use futures::sync::oneshot;
use hyper::header::ContentLength;
use hyper::server::{Http, Request, Response, Service};
use tokio_core::reactor::{Core, Handle};
use aggregator::merkle::Sha256Hash;
use aggregator::merkle::{aggregator_start, sha256};


fn main() {
    let (tx_digest, rx_digest) = mpsc::channel();
    let (tx_future, rx_future) = mpsc::channel();

    let addr = "127.0.0.1:3000".parse().unwrap();
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let tx_digest_2 = tx_digest.clone();
    let handle_2 = handle.clone();
    let arc_rx_future = Arc::new(rx_future);
    let server = Http::new().serve_addr_handle(&addr, &handle,move || Ok(
        AggregatorServerData::new(tx_digest_2.clone(), handle_2.clone(), arc_rx_future.clone())
    )).unwrap();

    let handle_3 = handle.clone();
    handle.spawn(server.for_each(move |conn| {
        handle_3.spawn(conn.map(|_| ()).map_err(|err| println!("serve error: {:?}", err)));
        Ok(())
    }).map_err(|_| ()));

    thread::spawn(move || {
        aggregator_start(rx_digest, tx_future);
    });

    core.run(futures::future::empty::<(), ()>()).unwrap();
}

struct AggregatorServerData {
    tx_digest : mpsc::Sender<Sha256Hash>,
    handle: Handle,
    rx_future: Arc<mpsc::Receiver<oneshot::Receiver<u32>>>,
}

impl AggregatorServerData {
    fn new(tx_digest : mpsc::Sender<Sha256Hash>, handle : Handle, rx_future: Arc<mpsc::Receiver<oneshot::Receiver<u32>>>) -> AggregatorServerData {
        AggregatorServerData {
            tx_digest,
            handle,
            rx_future,
        }
    }
}

impl Service for AggregatorServerData {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let digest = get_digest(req);

        self.tx_digest.send(digest).unwrap();

        let result_receiver = self.rx_future.recv().unwrap();
        //println!("result_receiver:{:?}",result_receiver);

        Box::new(
            result_receiver.map(|res| {
            //println!("result_receiver.map:{:?}",res);
            let res = format!("{}",res);
                Response::new()
                .with_header(ContentLength(res.len() as u64))
                .with_body(res)

            }).map_err(|_| hyper::Error::Incomplete)
        )
    }
}

fn get_digest(_req: Request) -> Sha256Hash {
    let mut rng = rand::thread_rng();
    let mut bytes=[0u8;44];
    rng.fill_bytes(&mut bytes);
    sha256(&bytes)
}
