extern crate futures;
extern crate tokio;
extern crate tokio_core;
extern crate hyper;

use tokio_core::reactor::Core;
use std::time::Duration;
use std::time::Instant;
use futures::Stream;
use tokio_core::reactor::Interval;

fn main() {

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let interval = Interval::new(Duration::from_millis(100), &handle).unwrap();
    let task = interval.for_each(|_| {
        println!("now {:?}", Instant::now());
        futures::future::ok(())
    });
    core.run(task).unwrap();

}
