use std::sync::Arc;
use std::sync::Mutex;
use hyper;
use hyper::server::{Request, Response, Service};
use hyper::{Post, StatusCode};
use futures::sync::oneshot;
use futures::{self, Stream, Future};
use futures::sync::oneshot::Sender;
use data_encoding::HEXLOWER;
use timestamp::LinearTimestamp;
use opentimestamps::op::Op;
use rand::prelude::thread_rng;
use rand::Rng;

pub struct Aggregator {
    // could be Rc<RefCell<...> at the moment, but what about the new multithreaded eventloop?
    pub requests_to_serve : Arc<Mutex<RequestsToServe>>,
}

pub struct RequestsToServe {
    requests : Vec<RequestToServe>,
}

pub struct RequestToServe {
    pub timestamp: LinearTimestamp,
    pub sender: Sender<Vec<u8>>,
}

impl RequestToServe {
    fn new(timestamp: LinearTimestamp, sender: Sender<Vec<u8>>) -> RequestToServe {
        RequestToServe {
            timestamp,
            sender,
        }
    }
}

impl RequestsToServe {
    pub fn push(&mut self, request_to_serve : RequestToServe) {
        self.requests.push(request_to_serve);
    }
    pub fn len(&self) -> usize {
        self.requests.len()
    }
    pub fn pop(&mut self) -> Option<RequestToServe> {
        self.requests.pop()
    }
}

impl Default for RequestsToServe {
    fn default() -> RequestsToServe {
        RequestsToServe {
            requests : Vec::new(),
        }
    }
}

const MAX_DIGEST_LENGTH : usize = 64;

impl Service for Aggregator {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        match (req.method(), req.path()) {
            (&Post, "/digest") => {
                let mut requests_to_serve_cloned = self.requests_to_serve.clone();
                let (sender, receiver) = oneshot::channel();
                let future = req.body()
                    .concat2()
                    .and_then(move |chunk| {
                        let digest = chunk.iter()
                            .cloned()
                            .collect::<Vec<u8>>();
                        if digest.len() > MAX_DIGEST_LENGTH {
                            return Err(hyper::Error::Incomplete);
                        }
                        debug!("Received digest {}", HEXLOWER.encode(&digest));
                        let mut timestamp = LinearTimestamp::new(digest);
                        timestamp.push(Op::Append(nonce()));
                        timestamp.push(Op::Sha256);
                        debug!("{}", timestamp);
                        let mut requests_to_serve = requests_to_serve_cloned.lock().unwrap();
                        requests_to_serve.push(RequestToServe::new(timestamp, sender));
                        Ok(())
                    })
                    .join(receiver.map_err(|e| { println!("{:?}",e); hyper::Error::Incomplete }))
                    .map(|result| {
                        Response::new().with_body(result.1.to_vec())
                    }).map_err(|_| hyper::Error::Incomplete);

                Box::new(future)
            },
            _ => {
                Box::new(futures::future::ok(
                    Response::new().with_status(StatusCode::NotFound)
                ))
            },
        }
    }
}


fn nonce() -> Vec<u8> {
    let mut vec: Vec<u8> = Vec::with_capacity(16);
    let mut rng = thread_rng();
    for _ in 0..16 {
        vec.push(rng.gen());
    }
    vec
}