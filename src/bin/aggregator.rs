extern crate ots_aggregator;
extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate data_encoding;
extern crate clap;
extern crate env_logger;

#[macro_use]
extern crate log;

use tokio_core::reactor::{Interval, Core};
use std::time::{Instant, Duration};
use hyper::{Post};
use hyper::header::ContentLength;
use hyper::server::{Http, Request};
use futures::{Stream, Future};
use std::sync::{Mutex, Arc};
use hyper::{Chunk, Client, Uri};
use ots_aggregator::merkle::{sha256, sha256_two_input};
use futures::sync::oneshot::Sender;
use ots_aggregator::server::{Aggregator, RequestsToServe};
use ots_aggregator::merkle::Sha256Hash;
use ots_aggregator::{merkle, Millis};
use data_encoding::HEXLOWER;
use std::collections::HashMap;
use clap::{Arg, App};
use std::net::SocketAddr;

fn main() {
    env_logger::init();
    let matches  = App::new("OpenTimestamps Aggregator")
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(
            Arg::with_name("bind")
                .short("b")
                .long("bind")
                .takes_value(true)
                .help("Address to bind (default: 127.0.0.1:1337)")
        )
        .arg(
            Arg::with_name("calendar")
                .short("c")
                .long("calendar")
                .takes_value(true)
                .help("Address of the calendar server")
        )
        .arg(
            Arg::with_name("time-slice")
                .short("s")
                .long("time-slice")
                .takes_value(true)
                .help("Time slice of the aggregation in ms (default: 100ms)")
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .takes_value(false)
                .help("If present log in debug mode")
        )
        .get_matches();

    let addr = matches.value_of("bind").unwrap_or("127.0.0.1:1337").parse::<SocketAddr>().expect("Invalid bind address");
    let calendar = matches.value_of("calendar").expect("Address of the back-end calendar is mandatory, specify it with --calendar");
    let time_slice = matches.value_of("time-slice").unwrap_or("100").parse().unwrap_or(100u64);

    println!("Starting on {:?}, using backend calendar at {} with a time slice of {}ms", addr, calendar, time_slice);

    let uri : Uri = calendar.parse().expect("Address of the back-end calendar does not parse");
    let requests_to_serve = Arc::new(Mutex::new(RequestsToServe::default()));

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let requests_to_serve_cloned = requests_to_serve.clone();
    let server = Http::new().serve_addr_handle(&addr, &handle, move|| Ok(
        Aggregator{requests_to_serve : requests_to_serve_cloned.clone()}
    )).unwrap();

    let handle_2 = handle.clone();
    handle.spawn(server.for_each(move |conn| {
        handle_2.spawn(conn.map(|_| ()).map_err(|err| println!("serve error: {:?}", err)));
        Ok(())
    }).map_err(|_| ()));

    let handle_3 = handle.clone();
    let client = Client::new(&handle_3);
    let requests_to_serve_cloned = requests_to_serve.clone();
    let interval = Interval::new(Duration::from_millis(time_slice), &handle).unwrap();
    let task = interval.for_each(move|_| {
        let mut requests_to_serve = requests_to_serve_cloned.lock().unwrap();
        let total_requests = requests_to_serve.len();
        if total_requests > 0 {
            debug!("Requests_to_serve: {:?}", total_requests);
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
            let future = client.request(req)
                .and_then(move|res| {
                    println!("Response from calendar: {} elapsed: {}ms", res.status(), start.elapsed().as_millis());
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

fn answer(merkle_proofs : HashMap<Sha256Hash, Vec<u8>>, digests : Vec<Sha256Hash>, mut senders : Vec<Sender<Vec<u8>>>, body : Chunk) {
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
        senders.pop().unwrap().send(response).unwrap();  // first unwrap because senders vec has same elements of digests, second unwrap to handle
    }
}