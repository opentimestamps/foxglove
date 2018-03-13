extern crate futures;
extern crate tokio_core;

use std::sync::mpsc;

use futures::future;
use futures::Future;
use futures::sync::oneshot;
use tokio_core::reactor::Core;


fn main() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();


    let future = future::ok::<(),()>(());
    let (tx, rx) = mpsc::channel();
    tx.send(future);
    let r = rx.recv().unwrap();
    //println!("{:?}",r);

    let (tx_for_oneshot, rx_for_oneshot) = mpsc::channel();
    let (tx_oneshot, rx_oneshot) = oneshot::channel();

    tx_for_oneshot.send(tx_oneshot);
    let r = rx_for_oneshot.recv().unwrap();
    r.send(1);
    let result = rx_oneshot.then(|res| { println!("{:?}",res.unwrap()); Ok::<_, ()>(())
    });

    core.run(result).unwrap();

    println!("finish");
}