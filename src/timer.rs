
use std::time::{Instant, Duration};
use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use tokio_core::reactor::{Interval, Core};
use tokio_core::reactor::Handle;
use hyper::{Chunk, Client, Uri};
use hyper::server::Request;
use hyper::Post;
use hyper::header::ContentLength;
use data_encoding::HEXLOWER;
use futures;
use futures::sync::oneshot::Sender;
use futures::{Stream, Future};
use Millis;
use merkle;
use merkle::{Sha256Hash, sha256, sha256_two_input};
use server::RequestsToServe;
use hyper_tls::HttpsConnector;


pub fn tick(
    handle : &Handle,
    requests_to_serve : Arc<Mutex<RequestsToServe>>,
    time_slice : u64,
    uri : Uri,
    core : &mut Core) {

    let https_client = Client::configure()
            .connector(HttpsConnector::new(4, &handle).unwrap())
            .build(&handle);

    let interval = Interval::new(Duration::from_millis(time_slice), &handle).unwrap();
    let task = interval.for_each(move|_| {
        let mut requests_to_serve = requests_to_serve.lock().unwrap();
        let total_requests = requests_to_serve.len();
        if total_requests > 0 {
            debug!("Requests_to_serve: {:?} nanos: {:?}", total_requests, Instant::now());
            let mut senders = Vec::new();
            let mut digests = Vec::new();
            while let Some(request_to_serve) = requests_to_serve.pop() {
                //sender.send([0u8].to_vec());
                senders.push(request_to_serve.sender);
                digests.push(request_to_serve.digest_sha256);
            }

            let (root, merkle_proofs) = merkle::make(&digests );
            let mut req : Request = Request::new(Post, uri.clone());
            let body = root.0.to_vec();
            req.headers_mut().set(ContentLength(body.len() as u64));
            req.set_body(body);
            let start = Instant::now();
            let future = https_client.request(req)
                .and_then(move|res| {
                    println!("Response from calendar: {} elapsed: {}ms",
                             res.status(), start.elapsed().as_millis());
                    res.body().concat2()
                })
                .and_then(move |body| {
                    debug!("Body: {} ", HEXLOWER.encode(&body) );
                    answer(merkle_proofs, digests, senders, body);
                    Ok(())
                })
                .map_err(|_| ());

            handle.spawn(future);
        }
        futures::future::ok(())
    });

    core.run(task).unwrap();
}


fn answer(
    merkle_proofs : HashMap<Sha256Hash, Vec<u8>>,
    digests : Vec<Sha256Hash>,
    mut senders : Vec<Sender<Vec<u8>>>,
    body : Chunk) {
    for digest in digests {
        let mut response : Vec<u8> = Vec::new();
        let mut current_hash = digest.clone();
        response.push(merkle::SHA256_TAG);  // first op on digest is sha256
        while let Some(result) = merkle_proofs.get(&current_hash) {
            debug!("extending {:?}", HEXLOWER.encode(&result));
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

        response.extend(body.to_vec());
        debug!("For {} returning: {}", HEXLOWER.encode(&digest.0), HEXLOWER.encode(&response));

        // first unwrap safe because senders vec has same elements of digests
        senders.pop().unwrap().send(response).unwrap();
    }
}