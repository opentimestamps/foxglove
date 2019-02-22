
use std::time::{Instant, Duration};
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
use merkle;
use merkle::Sha256Hash;
use server::RequestsToServe;
use hyper_tls::HttpsConnector;
use timestamp::MerklePaths;
use timestamp::LinearTimestamp;


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

        let mut timestamps = Vec::new();
        let mut senders = Vec::new();
        let mut digests = Vec::new();

        {   // using this block to fastly unlock the request_to_serve Arc
            let mut requests_to_serve = requests_to_serve.lock().unwrap();
            debug!("Requests_to_serve: {:?}", requests_to_serve.len());

            while let Some(request_to_serve) = requests_to_serve.pop() {
                senders.push(request_to_serve.sender);
                digests.push(Sha256Hash::from_vec( request_to_serve.timestamp.execute() ).unwrap() );
                timestamps.push(request_to_serve.timestamp);
            }
        }

        if digests.len()>0 {
            let (root, merkle_paths) = merkle::make(&digests );
            extend_timestamps(&mut timestamps, merkle_paths);
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
                    println!("Body: {} ", HEXLOWER.encode(&body) );
                    answer(timestamps, senders, body);
                    Ok(())
                })
                .map_err(|_| ());

            handle.spawn(future);
        }

        futures::future::ok(())
    });

    core.run(task).unwrap();
}

fn extend_timestamps(timestamps : &mut Vec<LinearTimestamp>, merkle_paths : MerklePaths) {
    for timestamp in timestamps {
        let mut current_hash = Sha256Hash::from_vec( timestamp.execute() ).unwrap();
        while let Some(ops) = merkle_paths.get(&current_hash) {
            timestamp.extend(ops.clone());
            current_hash = Sha256Hash::from_vec(ops.execute(current_hash.0.to_vec())).unwrap();
        }
        debug!("extend timestamp {}", timestamp);
    }
}

fn answer(
    timestamps : Vec<LinearTimestamp>,
    mut senders : Vec<Sender<Vec<u8>>>,
    body : Chunk) {

    for timestamp in timestamps.iter().rev() {
        let mut response : Vec<u8> = Vec::new();
        response.extend(timestamp.ops.serialize() );
        response.extend(body.to_vec());
        debug!("For {} returning: {}",
               HEXLOWER.encode(&timestamp.initial_msg),
               HEXLOWER.encode(&response));
        // first unwrap safe because senders vec has same elements of digests
        senders.pop().unwrap().send(response).unwrap();
    }
    println!("-----");
}