use std::sync::mpsc;
use std::time::{Instant, Duration };
use std::collections::HashMap;
use futures::{self, Future, Sink};
use futures::sync::oneshot;
use tokio_core::reactor::Core;
use tokio_core::reactor::Remote;
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use hyper::Method::Post;
use hyper::server::Request;
use hyper::header::ContentLength;
use client::RequestAndClientsFuture;
use Millis;

const TIME_SLICE_MILLIS: u64 = 200;
const THREAD_RECV_MILLIS: u64 = 2;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Sha256Hash(pub [u8; 32]);

pub fn aggregator_start(rx_digest : mpsc::Receiver<Sha256Hash>,
                        tx_future : mpsc::Sender<oneshot::Receiver<u32>>,
                        tx_request : mpsc::Sender<RequestAndClientsFuture>,
) {
    let time_slice_millis: Duration = Duration::from_millis(TIME_SLICE_MILLIS);
    let thread_recv_millis: Duration = Duration::from_millis(THREAD_RECV_MILLIS);
    let mut i: u64 = 0;
    let mut start_cycle: Option<Instant> = None;
    let mut current_round_hashes : Vec<Sha256Hash> = Vec::new();
    let mut current_round_senders : Vec<(Sha256Hash, oneshot::Sender<u32>)> =  Vec::new();

    println!("Started merkle");
    loop {
        if start_cycle.is_some() && start_cycle.unwrap().elapsed() >= time_slice_millis {
            start_cycle = None;
            let now = Instant::now();
            println!("Creating Merkle of {} elements", current_round_hashes.len());

            // Create merkle tree
            let mut merkle_proofs : HashMap<Sha256Hash, Vec<u8>> = HashMap::new();
            let root = merkle_root_and_paths(&current_round_hashes, &mut merkle_proofs);
            println!("Elapsed {}ms, root is {:?}", now.elapsed().as_millis(), root);
            let mut senders : Vec<(Sha256Hash, oneshot::Sender<u32>)> = current_round_senders;

            tx_request.send(RequestAndClientsFuture::new(root, merkle_proofs, senders));
            println!("Sent!");

            current_round_hashes = vec!();
            current_round_senders = vec!();
        }

        if let Ok(result) = rx_digest.recv_timeout(thread_recv_millis) {   //TODO suboptimal, should be a future timeout?
            if start_cycle.is_none() {
                start_cycle = Some(Instant::now());
            }
            let (tx_oneshot, rx_oneshot) = oneshot::channel();
            tx_future.send(rx_oneshot).unwrap();
            current_round_hashes.push(result.clone());
            current_round_senders.push((result,tx_oneshot));
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
