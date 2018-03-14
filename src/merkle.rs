use std::sync::mpsc;
use std::time::{Instant, Duration };
use std::collections::HashMap;
use futures::sync::oneshot;
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use client::RequestAndClientsFuture;
use Millis;
use data_encoding::HEXLOWER;
use std::fmt::Formatter;
use std::fmt;

const TIME_SLICE_MILLIS: u64 = 1000;
const THREAD_RECV_MILLIS: u64 = 2;

pub const SHA256_TAG : u8 = 0x08;
pub const APPEND_TAG : u8 = 0xf0;
pub const PREPEND_TAG : u8 = 0xf1;
pub const SHA256_SIZE : u8 = 0x20;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Sha256Hash(pub [u8; 32]);

impl fmt::Display for Sha256Hash {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", HEXLOWER.encode(&self.0))
    }
}


pub fn aggregator_start(rx_digest : mpsc::Receiver<Sha256Hash>,
                        tx_future : mpsc::Sender<oneshot::Receiver<Vec<u8>>>,
                        tx_request : mpsc::Sender<RequestAndClientsFuture>,
) {
    let time_slice_millis: Duration = Duration::from_millis(TIME_SLICE_MILLIS);
    let thread_recv_millis: Duration = Duration::from_millis(THREAD_RECV_MILLIS);
    let mut i: u64 = 0;
    let mut start_cycle: Option<Instant> = None;
    let mut current_round_hashes : Vec<Sha256Hash> = Vec::new();
    let mut current_round_senders : Vec<(Sha256Hash, oneshot::Sender<Vec<u8>>)> =  Vec::new();

    println!("Started merkle thread");
    loop {
        if start_cycle.is_some() && start_cycle.unwrap().elapsed() >= time_slice_millis {
            start_cycle = None;
            let now = Instant::now();
            let elements = current_round_hashes.len();

            // Create merkle tree
            let mut merkle_proofs : HashMap<Sha256Hash, Vec<u8>> = HashMap::new();
            let root = merkle_root_and_paths(&current_round_hashes, &mut merkle_proofs);
            println!("merkle of #{} elapsed {:.3}ms, root {}", elements, now.elapsed().as_millis(), root);
            tx_request.send(RequestAndClientsFuture::new(root, merkle_proofs, current_round_senders)).unwrap();

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
    let sha256_tag = vec![SHA256_TAG];
    let append_tag = vec![APPEND_TAG];
    let prepend_tag = vec![PREPEND_TAG];
    let sha256_size = vec![SHA256_SIZE];
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
                Some(next) =>  merkle_proofs.insert(Sha256Hash(el.0),merge_4_slices(&append_tag, &sha256_size , &next.0, &sha256_tag)),
                None => merkle_proofs.insert(Sha256Hash(el.0),sha256_tag.clone()),
            };
        } else {
            merkle_proofs.insert(Sha256Hash(el.0),merge_4_slices(&prepend_tag, &sha256_size, &hash_list[i-1].0, &sha256_tag));
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

#[inline]
pub fn merge_4_slices(a: &[u8], b: &[u8], c: &[u8], d: &[u8]) -> Vec<u8> {
    let mut r = merge_3_slices(a,b,c);
    r.extend(d.to_vec());
    r
}