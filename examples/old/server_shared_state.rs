extern crate futures;
extern crate hyper;

use futures::future::FutureResult;

use hyper::{Get, Post, StatusCode};
use hyper::header::ContentLength;
use hyper::server::{Http, Service, Request, Response};
use futures::Stream;
use futures::Future;
use std::sync::Mutex;
use std::sync::Arc;
use hyper::Chunk;

pub struct Echo {
    state : Arc<Mutex<Vec<u8>>>,
    timeout : Arc<Mutex<Timeout>>,
}

impl Service for Echo {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        match (req.method(), req.path()) {
            (&Get, "/") => {
                Box::new(futures::future::ok(
                    Response::new().with_body("Try POSTing data to /echo")
                ))
            },
            (&Get, "/timeout") => {
                let mut timeout_cloned = self.timeout.clone();
                let mut timeout_locked = timeout_cloned.lock().unwrap();
                match timeout_locked.poll() {
                    Async::NotReady(_) => println!("NotReady"),
                    Async::Ready(_) => {
                        println!("Ready");
                        timeout_locked.reset(Instant::now() + Duration::from_secs(10))
                    },
                }
                Box::new(futures::future::ok(
                    Response::new().with_body("Timeout")
                ))
            },
            (&Post, "/reverse") => {
                Box::new(
                    req.body()
                        .concat2()
                        .map(|chunk| {
                            let reversed = chunk.iter()
                                .rev()
                                .cloned()
                                .collect::<Vec<u8>>();
                            Response::new()
                                .with_body(reversed)
                        })
                )
            },
            (&Post, "/concat") => {
                let mut state_cloned = self.state.clone();

                Box::new(
                    req.body()
                        .concat2()
                        .map(move |chunk| {
                            let data = chunk.iter()
                                .cloned()
                                .collect::<Vec<u8>>();
                            let mut state_locked = state_cloned.lock().unwrap();
                            state_locked.extend(data);
                            Response::new()
                                .with_body(format!("{:?}",state_moved.to_vec()))
                        })
                )
            },
            _ => {
                Box::new(futures::future::ok(
                    Response::new().with_status(StatusCode::NotFound)
                ))
            },
        }
    }
    /*
    fn call(&self, req: Request) -> Self::Future {

        Box::new(
            req.body().concat2().map(|chunk| {
                let body = chunk.to_vec();
                println!("Body is {:?}", body);

                Response::new()
                    .with_header(ContentLength(body.len() as u64))
                    .with_body(body)
            })
        )
    }*/

}

fn reverse(chunk: Chunk) -> Response {
    let reversed = chunk.iter()
        .rev()
        .cloned()
        .collect::<Vec<u8>>();
    Response::new()
        .with_body(reversed)
}


fn main() {
    let addr = "127.0.0.1:1337".parse().unwrap();
    let state : Vec<u8>= Vec::new();
    let state = Arc::new(Mutex::new(state));
    let timeout = Timeout::new(Duration::from_secs(10), &handle).unwrap();
    let timeout = Arc::new(Mutex::new(timeout));

    let server = Http::new().bind(&addr,  move|| Ok(Echo{state : state.clone(), timeout : timeout.clone()})).unwrap();

    server.run().unwrap();
}
