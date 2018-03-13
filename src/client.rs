use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Instant;
use tokio_core::reactor::{Core, Handle};
use tokio_core::reactor::Remote;
use hyper::server::Request;
use hyper::Method::Post;
use hyper::header::ContentLength;
use hyper::{Client, Uri};
use futures;
use futures::future;
use futures::Stream;
use futures::Future;
use futures::sync::oneshot;
use merkle::Sha256Hash;
use Millis;
use std::thread;

static URI: &str = "http://163.172.157.16:14732/digest";

pub struct RequestAndClientsFuture {
    root : Sha256Hash,
    merkle_paths: HashMap<Sha256Hash, Vec<u8>>,
    client_futures: Vec<(Sha256Hash, oneshot::Sender<u32>)>
}

impl RequestAndClientsFuture {
    pub fn new(root : Sha256Hash,
               merkle_paths: HashMap<Sha256Hash, Vec<u8>>,
               client_futures: Vec<(Sha256Hash, oneshot::Sender<u32>)>) -> RequestAndClientsFuture {
        RequestAndClientsFuture {
            root,
            merkle_paths,
            client_futures,
        }
    }
}

pub fn start( rx : mpsc::Receiver<RequestAndClientsFuture>) {



    println!("Started client");
    loop {
        let req_and_clients = rx.recv().unwrap();
        let uri : Uri = URI.parse().unwrap();
        println!("Received req_and_clients");


        /// I should better use futures, but I have problems in handling the
        /// executore handle moving around threads... And launching the future like
        /// core.run(work).unwrap(); which blocks subsequent request
        /// In the meantime I spawn a thread which is bad because require a new thread every
        /// time and because create a client every time not leveragin the keep-alive

        thread::spawn(move || {
            let mut core = Core::new().unwrap();
            let client = Client::new(&core.handle());

            let mut req = Request::new(Post, uri.clone());
            let body = req_and_clients.root.0.to_vec();
            req.headers_mut().set(ContentLength(body.len() as u64));
            req.set_body(body);
            let start = Instant::now();
            let work = client.request(req).map(move |res| {
                println!("Response: {} elapsed: {}ms", res.status(), start.elapsed().as_millis());
                let mut req_and_clients = req_and_clients;
                while let Some(client_future) = req_and_clients.client_futures.pop() {
                    //TODO here I should build the proper request
                    client_future.1.send(0);
                }
            }).map_err(|_| ());

            core.run(work).unwrap();
        });
    }

}
