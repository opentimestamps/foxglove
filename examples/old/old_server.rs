extern crate ots_aggregator;
extern crate hyper;
extern crate futures;
extern crate rand;
extern crate tokio_core;
extern crate crypto;

use std::thread;
use std::sync::{mpsc, Arc};
use futures::Stream;
use futures::future::Future;
use hyper::server::Http;
use tokio_core::reactor::Core;
use ots_aggregator::merkle;
use ots_aggregator::server::AggregatorServerData;
use ots_aggregator::client;

fn main() {

    // every input digest sent from the server to the merkle_aggregator through this channel
    let (tx_digest, rx_digest) = mpsc::channel();

    // every digest sent to the merkle_aggregator return immediately a future result through this channel
    let (tx_future, rx_future) = mpsc::channel();

    // every http request to the back calendar is sent through this channel
    let (tx_request, rx_request) = mpsc::channel();

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    // SERVER
    let addr = "127.0.0.1:3000".parse().unwrap();
    let tx_digest_2 = tx_digest.clone();
    let arc_rx_future = Arc::new(rx_future);
    let server = Http::new().serve_addr_handle(&addr, &handle,move || Ok(
        AggregatorServerData::new(tx_digest_2.clone(), arc_rx_future.clone())
    )).unwrap();
    let handle_2 = handle.clone();
    handle.spawn(server.for_each(move |conn| {
        handle_2.spawn(conn.map(|_| ()).map_err(|err| println!("serve error: {:?}", err)));
        Ok(())
    }).map_err(|_| ()));
    println!("Started server");

    // MERKLE
    thread::spawn(move || {
        merkle::aggregator_start(rx_digest, tx_future, tx_request);
    });

    // CLIENT
    thread::spawn(move || {
        client::start(rx_request)
    });

    core.run(futures::future::empty::<(), ()>()).unwrap();
}
