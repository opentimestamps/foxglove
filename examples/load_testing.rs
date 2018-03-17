extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate rand;

use std::io::{self, Write};
use futures::{Future, Stream};
use hyper::Client;
use tokio_core::reactor::Core;
use hyper::{Method, Request};
use hyper::header::{ContentLength, ContentType};
use rand::Rng;
use std::env;
use hyper::Uri;

// http://163.172.157.16:14732/digest
// http://127.0.0.1:1337/digest

fn main() {
    if let Some(arg1) = env::args().nth(1) {
        println!("The first argument is {}", arg1);
        start(arg1);
    } else{
        println!("Specify the aggregator address");
    }
}

fn start( cal : String) -> Result<(),()> {
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::new(&handle );

    let uri: Uri = cal.parse().unwrap();

    let mut bytes = [0u8;44];
    let mut rng = rand::thread_rng();

    for i in 0..100 {
        let mut req : Request = Request::new(Method::Post, uri.clone());
        rng.fill_bytes(&mut bytes);
        req.headers_mut().set(ContentType::octet_stream());
        req.headers_mut().set(ContentLength(bytes.len() as u64));
        req.set_body(bytes.to_vec());
        let post = client.request(req)
            .and_then(|res| {
                //println!("POST: {}", res.status());
                res.body().concat2()
            })
            .map(|_res| ())
            .map_err(|_| ());

        handle.spawn(post);
    }

    core.run(futures::future::empty::<(), ()>()).unwrap();

    Ok(())
}
