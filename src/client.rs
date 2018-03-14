use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Instant;
use tokio_core::reactor::Core;
use hyper::server::Request;
use hyper::Method::Post;
use hyper::header::ContentLength;
use hyper::{Client, Uri};
use futures::Future;
use futures::sync::oneshot;
use merkle::Sha256Hash;
use merkle;
use Millis;
use std::thread;
use futures::Stream;
use merkle::sha256;
use merkle::sha256_two_input;

static URI: &str = "http://163.172.157.16:14732/digest";

pub struct RequestAndClientsFuture {
    root : Sha256Hash,
    merkle_paths: HashMap<Sha256Hash, Vec<u8>>,
    client_futures: Vec<(Sha256Hash, oneshot::Sender<Vec<u8>>)>
}

impl RequestAndClientsFuture {
    pub fn new(root : Sha256Hash,
               merkle_paths: HashMap<Sha256Hash, Vec<u8>>,
               client_futures: Vec<(Sha256Hash, oneshot::Sender<Vec<u8>>)>) -> RequestAndClientsFuture {
        RequestAndClientsFuture {
            root,
            merkle_paths,
            client_futures,
        }
    }
}

pub fn start( rx : mpsc::Receiver<RequestAndClientsFuture>) {
    println!("Started client thread");
    loop {
        let req_and_clients = rx.recv().unwrap();
        let uri : Uri = URI.parse().unwrap();

        // I should better use futures, but I have problems in handling the
        // executore handle moving around threads... And launching the future like
        // core.run(work).unwrap(); which blocks subsequent request
        // In the meantime I spawn a thread which is bad because require a new thread every
        // time and because create a client every time not leveragin the keep-alive

        thread::spawn(move || {
            let mut core = Core::new().unwrap();
            let client = Client::new(&core.handle());

            let mut req = Request::new(Post, uri.clone());
            let body = req_and_clients.root.0.to_vec();
            req.headers_mut().set(ContentLength(body.len() as u64));
            req.set_body(body);
            let start = Instant::now();
            let work = client
                    .request(req)
                    .and_then(|res| {
                        res.body().concat2().map(move |body| {
                            println!("Response length: {} elapsed: {}ms", body.len() , start.elapsed().as_millis());
                            let mut req_and_clients = req_and_clients;
                            let merkle_paths = req_and_clients.merkle_paths;
                            let body = body.to_vec();
                            while let Some(client_future) = req_and_clients.client_futures.pop() {
                                let (hash, sender) = client_future;
                                let mut response : Vec<u8> = Vec::new();
                                response.push(merkle::SHA256_TAG);  // first op on digest is sha256
                                let mut current_hash = Sha256Hash(hash.0);
                                while let Some(result) = merkle_paths.get(&current_hash) {
                                    //println!("extending {:?}", HEXLOWER.encode(&result));
                                    current_hash = match result[0] {
                                        merkle::SHA256_TAG => {
                                            sha256(&current_hash.0)
                                        },
                                        merkle::APPEND_TAG => {
                                            sha256_two_input(&current_hash.0, &result[2..result.len()-1])
                                        },
                                        merkle::PREPEND_TAG => {
                                            sha256_two_input(&result[2..result.len()-1], &current_hash.0)
                                        },
                                        _ => {
                                            panic!("Unexpected TAG");
                                        }
                                    };
                                    response.extend(result);
                                }
                                response.extend(body.clone());
                                //println!("hash(received): {} response: {}", hash, HEXLOWER.encode(&response));
                                sender.send(response).unwrap();
                            }
                        })
                    });

            core.run(work).unwrap();
        });
    }

}
