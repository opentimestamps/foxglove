extern crate foxglove;
extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio_core;
extern crate data_encoding;
extern crate clap;
extern crate env_logger;

// #[macro_use]
extern crate log;

use std::net::SocketAddr;
use std::sync::{Mutex, Arc};
use tokio_core::reactor::Core;
use hyper::server::Http;
use hyper::Uri;
use futures::{Stream, Future};
use clap::{Arg, App};
use foxglove::server::{Aggregator, RequestsToServe};
use foxglove::timer;

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
                .help("Address of the calendar server (default: https://finney.calendar.eternitywall.com/digest)")
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
                .help("Do nothing for now, define RUST_LOG=debug for debug output")
        )
        .get_matches();


    let addr = matches.value_of("bind").unwrap_or("127.0.0.1:1337").parse::<SocketAddr>()
        .expect("Invalid bind address");
    let calendar = matches.value_of("calendar").unwrap_or("https://finney.calendar.eternitywall.com/digest");
    let time_slice = matches.value_of("time-slice").unwrap_or("100").parse().unwrap_or(100u64);

    println!("Starting on {:?}, using backend calendar at {} with a time slice of {}ms",
             addr, calendar, time_slice);

    let uri : Uri = calendar.parse().expect("Address of the back-end calendar does not parse");
    let requests_to_serve = Arc::new(Mutex::new(RequestsToServe::default()));

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let requests_to_serve_2 = requests_to_serve.clone();
    let server = Http::new().serve_addr_handle(&addr, &handle, move|| Ok(
        Aggregator{requests_to_serve : requests_to_serve_2.clone()}
    )).unwrap();

    let handle_2 = handle.clone();
    handle.spawn(server.for_each(move |conn| {
        handle_2.spawn(conn.map(|_| ()).map_err(|err| println!("serve error: {:?}", err)));
        Ok(())
    }).map_err(|_| ()));

    let handle_3 = handle.clone();
    let requests_to_serve_3 = requests_to_serve.clone();
    timer::tick(&handle_3, requests_to_serve_3, time_slice, uri, &mut core);
}
