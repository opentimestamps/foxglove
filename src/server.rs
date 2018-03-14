use std::sync::{mpsc, Arc};
use futures::sync::oneshot;
use hyper;
use hyper::server::{Request, Response, Service};
use rand::{self, Rng};
use merkle::{Sha256Hash, sha256};
use futures::Future;
use hyper::header::ContentLength;

pub struct AggregatorServerData {
    tx_digest : mpsc::Sender<Sha256Hash>,
    rx_future: Arc<mpsc::Receiver<oneshot::Receiver<Vec<u8>>>>,
}

impl AggregatorServerData {
    pub fn new(tx_digest : mpsc::Sender<Sha256Hash>,
               rx_future: Arc<mpsc::Receiver<oneshot::Receiver<Vec<u8>>>>) -> AggregatorServerData {
        AggregatorServerData {
            tx_digest,
            rx_future,
        }
    }
}

impl Service for AggregatorServerData {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, _req: Request) -> Self::Future {
        //println!("Request {:?}", req);

        let digest = get_random_digest();

        self.tx_digest.send(digest).unwrap();

        let result_receiver = self.rx_future.recv().unwrap();
        //println!("result_receiver:{:?}",result_receiver);

        Box::new(
            result_receiver.map(|res| {
                //println!("result_receiver.map:{:?}",res);
                let res = format!("{:?}",res);
                Response::new()
                    .with_header(ContentLength(res.len() as u64))
                    .with_body(res)

            }).map_err(|_| hyper::Error::Incomplete)
        )
    }
}

/// this return a random digest, apparently, reading the body in a blocking way, even aware
/// of the shit perfomance is impossible to do
fn get_random_digest() -> Sha256Hash {
    let mut rng = rand::thread_rng();
    let mut bytes=[0u8;44];
    rng.fill_bytes(&mut bytes);
    let hash = sha256(&bytes);
    //println!("digest received {} which hashed is {}", HEXLOWER.encode(&bytes), hash);
    hash
}


