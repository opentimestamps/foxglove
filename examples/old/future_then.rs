extern crate aggregator;
extern crate futures;
extern crate tokio;
extern crate tokio_core;

use futures::sync::mpsc::unbounded;
use std::thread;
use futures::Stream;
use futures::Future;
use tokio::executor::current_thread;
use tokio_core::reactor::Core;

fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    println!("1");
    let first = futures::future::ok::<u32, ()>(1u32);
    println!("2");
    let second = first.clone().and_then(|one| Ok(one +1) );
    println!("3");
    let third = second.and_then(|one| { thread::sleep_ms(1000); Ok(one +3) } );
    let third = third.shared();

    let fourth = third.clone().and_then(|one| { thread::sleep_ms(100); Ok(*one +3) });

    let fourth_2 = third.clone().and_then(|one| { thread::sleep_ms(100); Ok(*one +300) });

    println!("{:?}", fourth.wait().unwrap() );
    println!("{:?}", fourth_2.wait().unwrap() );
    println!("4");

}
