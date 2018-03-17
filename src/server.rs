use std::sync::Arc;
use futures::sync::oneshot;
use hyper;
use hyper::server::{Request, Response, Service};
use merkle::{Sha256Hash, sha256};
use futures::{self, Stream, Future};
use hyper::{Post, StatusCode};
use std::sync::Mutex;
use futures::sync::oneshot::Sender;

pub struct Aggregator {
    // could be Rc<RefCell<...> at the moment, but what about the new multithreaded eventloop?
    pub requests_to_serve : Arc<Mutex<RequestsToServe>>,
}

pub struct RequestsToServe {
    requests : Vec<RequestToServe>,
}

pub struct RequestToServe {
    pub digest_sha256: Sha256Hash,
    pub sender: Sender<Vec<u8>>,
}

impl RequestToServe {
    fn new(digest_sha256: Sha256Hash, sender: Sender<Vec<u8>>) -> RequestToServe {
        RequestToServe {
            digest_sha256,
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
                        let digest_sha256 = sha256(&digest);
                        let mut requests_to_serve = requests_to_serve_cloned.lock().unwrap();
                        requests_to_serve.push(RequestToServe::new(digest_sha256, sender));
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


