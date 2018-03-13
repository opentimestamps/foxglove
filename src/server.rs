use std::sync::{mpsc, Arc};
use futures::sync::oneshot;
use hyper;
use hyper::server::{Http, Request, Response, Service};
use rand::{self, Rng};
use merkle::{Sha256Hash, sha256};
use tokio_core::reactor::{Core, Handle};
use futures::Future;
use hyper::header::ContentLength;
use tokio_core::reactor::Remote;


pub struct AggregatorServerData {
    tx_digest : mpsc::Sender<Sha256Hash>,
    handle: Handle,
    rx_future: Arc<mpsc::Receiver<oneshot::Receiver<u32>>>,
}

impl AggregatorServerData {
    pub fn new(tx_digest : mpsc::Sender<Sha256Hash>, handle : Handle, rx_future: Arc<mpsc::Receiver<oneshot::Receiver<u32>>>) -> AggregatorServerData {
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
