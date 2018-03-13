extern crate aggregator;
extern crate hyper;
extern crate futures;
extern crate rand;
extern crate tokio_core;
extern crate crypto;

use futures::future::Future;
use futures::future;
use hyper::header::ContentLength;
use hyper::server::{Http, Request, Response, Service};
use std::thread;
use rand::Rng;
use tokio_core::reactor::{Remote, Handle};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Instant, Duration };
use aggregator::FutureSender;
use futures::Sink;
use futures::Stream;
use tokio_core::reactor::Core;
use futures::sync::oneshot;
use std::collections::HashMap;
use crypto::sha2::Sha256;
use std::iter::Iterator;
use crypto::digest::Digest;
use hyper::Method::Post;
use hyper::Client;
use hyper::Uri;

const PHRASE: &'static str = "Hello, World!";
const TIME_SLICE_MILLIS: u64 = 200;
const THREAD_RECV_MILLIS: u64 = 2;
static URL: &str = "http://163.172.157.16:14732/digest";


fn main() {
    let (tx_digest, rx_digest) = mpsc::channel();
    let (tx_future, rx_future) = mpsc::channel();

    let addr = "127.0.0.1:3000".parse().unwrap();
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let tx_digest_2 = tx_digest.clone();
    let handle_2 = handle.clone();
    let arc_rx_future = Arc::new(rx_future);
    let server = Http::new().serve_addr_handle(&addr, &handle,move || Ok(
        AggregatorServerData::new(tx_digest_2.clone(), handle_2.clone(), arc_rx_future.clone())
    )).unwrap();

    let handle_3 = handle.clone();
    handle.spawn(server.for_each(move |conn| {
        handle_3.spawn(conn.map(|_| ()).map_err(|err| println!("serve error: {:?}", err)));
        Ok(())
    }).map_err(|_| ()));

    thread::spawn(move || {
        aggregator_start(rx_digest, tx_future);
    });

    core.run(futures::future::empty::<(), ()>()).unwrap();
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Sha256Hash([u8; 32]);

struct AggregatorServerData {
    tx_digest : mpsc::Sender<Sha256Hash>,
    handle: Handle,
    rx_future: Arc<mpsc::Receiver<oneshot::Receiver<u32>>>,
}

impl AggregatorServerData {
    fn new(tx_digest : mpsc::Sender<Sha256Hash>, handle : Handle, rx_future: Arc<mpsc::Receiver<oneshot::Receiver<u32>>>) -> AggregatorServerData {
        AggregatorServerData {
            tx_digest,
            handle,
            rx_future,
        }
    }
}

impl Service for AggregatorServerData {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let digest = get_digest(req);

        self.tx_digest.send(digest);

        let result_receiver = self.rx_future.recv().unwrap();
        //println!("result_receiver:{:?}",result_receiver);

        Box::new(
            result_receiver.map(|res| {
            //println!("result_receiver.map:{:?}",res);
            let res = format!("{}",res);
                Response::new()
                .with_header(ContentLength(res.len() as u64))
                .with_body(res)

            }).map_err(|_| hyper::Error::Incomplete)
        )
    }
}

fn get_digest(_req: Request) -> Sha256Hash {
    let mut rng = rand::thread_rng();
    let mut bytes=[0u8;44];
    rng.fill_bytes(&mut bytes);
    sha256(&bytes)
}

fn aggregator_start(rx_digest : mpsc::Receiver<Sha256Hash>, tx_future : mpsc::Sender<oneshot::Receiver<u32>>) {
    let time_slice_millis: Duration = Duration::from_millis(TIME_SLICE_MILLIS);
    let thread_recv_millis: Duration = Duration::from_millis(THREAD_RECV_MILLIS);
    let uri : Uri = URL.parse().unwrap();
    let mut core = Core::new().unwrap();
    let handle = core.handle();
    let mut i: u64 = 0;
    let mut start_cycle: Option<Instant> = None;
    let mut current_round_hashes = vec!();
    let mut current_round_senders = vec!();
    let client = Client::configure().build(&core.handle());

    loop {
        if start_cycle.is_some() && start_cycle.unwrap().elapsed() >= time_slice_millis {
            start_cycle = None;
            println!("Creating Merkle of {}|{} elements", current_round_hashes.len(),current_round_senders.len());

            // Create merkle tree
            let mut merkle_proofs : HashMap<Sha256Hash, Vec<u8>> = HashMap::new();
            let root = merkle_root_and_paths(&current_round_hashes, &mut merkle_proofs);
            println!("Root is {:?}", root);
            let hashes : Vec<Sha256Hash> = current_round_hashes;
            let mut senders : Vec<oneshot::Sender<u32>> = current_round_senders;

            //TODO send request

            let mut req = Request::new(Post, uri.clone() );
            let body = root.0.to_vec();
            req.headers_mut().set(ContentLength(body.len() as u64));
            req.set_body(body);
            //let web_res_future = client.request(req);
            let work = client.request(req).map(|res| {
                println!("{:?}",res);
            }).map_err(|err| {
                println!("{:?}",err);
            });

            /*
            let (tx, rx) = futures::sync::mpsc::channel(0);
            std::thread::spawn(move || {
                let mut core = Core::new().unwrap();
                let handle = core.handle();
                let client = Client::new(&handle);

                let messages = rx.for_each(|req| {
                    handle.spawn(client.request(req).and_then(do_something));
                    Ok(())
                });
                core.run(messages).unwrap();
            });

            // give the `tx` to someone else
            tx.send(Request::new(Method::Get, uri))`
            */
            let now = Instant::now();
            core.run(work);
            println!("ah {:?} ", now.elapsed() );
            // spawn future che nel then fa for e tutti i tx_oneshot shotta
            // ho probabilmente bisogno del future remote? In realtà nonn credo può essere un altro executor qua
            while let Some(sender) = senders.pop() {
                sender.send(0);
            }

            current_round_hashes = vec!();
            current_round_senders = vec!();


            //SEND RESULTS & clear map

            /*let mut keys: Vec<Sha256Hash> = vec!();  // this should be a oneliner
            for key in current_round.keys() {
                keys.push(Sha256Hash(key.0));
            }
            for key in keys {
                let tx_oneshot = current_round.remove(&key).unwrap();
                tx_oneshot.send(0);
            }*/

        }

        if let Ok(result) = rx_digest.recv_timeout(thread_recv_millis) {   //TODO should be a future timeout?
            if start_cycle.is_none() {
                start_cycle = Some(Instant::now());
            }
            let (tx_oneshot, rx_oneshot) = oneshot::channel();
            tx_future.send(rx_oneshot);
            current_round_hashes.push(result);
            current_round_senders.push(tx_oneshot);
            // println!("{:?}", rx_oneshot);
        }
        i = i + 1;

    }
}


pub fn merkle_root_and_paths(hash_list: &[Sha256Hash], merkle_proofs : &mut HashMap<Sha256Hash,Vec<u8>>) -> Sha256Hash {
    let sha256_tag = vec![8u8];
    let n_hashes = hash_list.len();
    if n_hashes == 1 {
        return Sha256Hash(hash_list.first().unwrap().0);
    }

    // Calculates sha hash for each pair. If len is odd, last value is hashed alone. ()
    let mut hash_pairs = hash_list.chunks(2)
        .map(|c| {
            if c.len()==2 {
                sha256_two_input(&c[0].0, &c[1].0)
            } else {
                sha256(&c[0].0)
            }
        })
        .collect::<Vec<Sha256Hash>>();

    // Insert paths to reach the next element
    for (i, el) in hash_list.iter().enumerate() {
        if i % 2 == 0 {
            match hash_list.get(i+1) {
                Some(next) =>  merkle_proofs.insert(Sha256Hash(el.0),merge_3_slices(&sha256_tag,&el.0, &next.0)),
                None => merkle_proofs.insert(Sha256Hash(el.0),merge_slices(&sha256_tag,&el.0)),
            };
        } else {
            merkle_proofs.insert(Sha256Hash(el.0),merge_3_slices(&sha256_tag, &hash_list[i-1].0, &el.0));
        };
    }

    return merkle_root_and_paths(&mut hash_pairs, merkle_proofs);
}


#[inline]
pub fn sha256(data: &[u8]) -> Sha256Hash {
    let mut out = [0u8; 32];
    let mut hasher = Sha256::new();
    hasher.input(data);
    hasher.result(&mut out);
    Sha256Hash(out)
}

#[inline]
pub fn sha256_two_input(a: &[u8], b: &[u8] ) -> Sha256Hash {
    let mut out = [0u8; 32];
    let mut hasher = Sha256::new();
    hasher.input(a);
    hasher.input(b);
    hasher.result(&mut out);
    Sha256Hash(out)
}


#[inline]
pub fn merge_slices(a: &[u8], b: &[u8]) -> Vec<u8> {
    let mut r = a.to_vec();
    r.extend(b.to_vec());
    r
}

#[inline]
pub fn merge_3_slices(a: &[u8], b: &[u8], c: &[u8]) -> Vec<u8> {
    let mut r = merge_slices(a,b);
    r.extend(c.to_vec());
    r
}
