extern crate futures;
extern crate tokio;
extern crate tokio_core;
extern crate hyper;

use futures::sync::mpsc::unbounded;
use std::thread;
use futures::Stream;
use futures::Future;
use tokio::executor::current_thread;
use tokio_core::reactor::Core;
use hyper::Client;
use std::time::Duration;
use tokio_core::reactor::Timeout;
use futures::future::Either;
use std::io;


fn main() {
    let url = "http://httpbin.org/ip".parse::<hyper::Uri>().unwrap();

    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let client = Client::new(&handle);

    let get = client.get(url).and_then(|res| res.body().concat2());

    let timeout = Timeout::new(Duration::from_millis(600), &handle).unwrap();

    let work = get.select2(timeout).then(|res| match res {
        Ok(Either::A((got, _timeout))) => Ok(got),
        Ok(Either::B((_timeout_error, _get))) => {
            Err(hyper::Error::Io(io::Error::new(
                io::ErrorKind::TimedOut,
                "Client timed out while connecting",
            )))
        }
        Err(Either::A((get_error, _timeout))) => Err(get_error),
        Err(Either::B((timeout_error, _get))) => Err(From::from(timeout_error)),
    });

    let got = core.run(work).unwrap();
    println!("{}", String::from_utf8_lossy(&got));

}
