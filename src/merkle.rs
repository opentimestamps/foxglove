use std::time::Instant;
use std::collections::HashMap;
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use Millis;
use data_encoding::HEXLOWER;
use std::fmt::Formatter;
use std::fmt;

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

pub fn make(digests_sha256 : &[Sha256Hash]) -> (Sha256Hash, HashMap<Sha256Hash, Vec<u8>>) {
    let now = Instant::now();
    let mut merkle_proofs : HashMap<Sha256Hash, Vec<u8>> = HashMap::new();
    let root = merkle_root_and_paths(digests_sha256, &mut merkle_proofs);
    println!("merkle of #{} elapsed {:.3}ms, root {}",
             digests_sha256.len(), now.elapsed().as_millis(), root);
    (root, merkle_proofs)
}

pub fn merkle_root_and_paths(
    hash_list: &[Sha256Hash],
    merkle_proofs : &mut HashMap<Sha256Hash,Vec<u8>>) -> Sha256Hash {

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
                Some(next) =>  merkle_proofs
                    .insert(Sha256Hash(el.0),
                            merge_4_slices(&append_tag, &sha256_size , &next.0, &sha256_tag)),
                None => merkle_proofs.insert(Sha256Hash(el.0),sha256_tag.clone()),
            };
        } else {
            merkle_proofs
                .insert(Sha256Hash(el.0),
                        merge_4_slices(&prepend_tag, &sha256_size, &hash_list[i-1].0, &sha256_tag));
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