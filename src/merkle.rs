use std::time::Instant;
use std::fmt::Formatter;
use std::fmt;
use crypto::sha2::Sha256;
use crypto::digest::Digest;
use data_encoding::HEXLOWER;
use opentimestamps::op::Op;
use timestamp::Ops;
use timestamp::MerklePaths;


#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Sha256Hash(pub [u8; 32]);

impl fmt::Display for Sha256Hash {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", HEXLOWER.encode(&self.0))
    }
}

impl Sha256Hash {
    pub fn from_vec(vec : Vec<u8>) -> Result<Sha256Hash,()> {
        match vec.len() {
            32 => {
                let mut array = [0u8; 32];
                array.copy_from_slice(&vec);
                Ok(Sha256Hash(array))
            },
            _ => Err(())
        }
    }
}

pub fn make(digests_sha256 : &[Sha256Hash]) -> (Sha256Hash, MerklePaths) {
    let now = Instant::now();
    let mut merkle_paths = MerklePaths::new();
    let root = merkle_root_and_paths(digests_sha256, &mut merkle_paths);
    println!("merkle of #{} elapsed {:.3}ms, root {}",
             digests_sha256.len(), now.elapsed().as_millis(), root);
    (root, merkle_paths)
}

pub fn merkle_root_and_paths(
    hash_list: &[Sha256Hash],
    merkle_paths : &mut MerklePaths) -> Sha256Hash {

    let n_hashes = hash_list.len();
    if n_hashes == 1 {
        return Sha256Hash(hash_list.first().unwrap().0);
    }

    // Calculates sha hash for each pair. If len is odd, last value is kept the same
    let mut hash_pairs = hash_list.chunks(2)
        .map(|c| {
            if c.len()==2 {
                sha256_two_input(&c[0].0, &c[1].0)
            } else {
                c[0].clone()
            }
        })
        .collect::<Vec<Sha256Hash>>();

    // Insert paths to reach the next element
    for (i, el) in hash_list.iter().enumerate() {
        if i % 2 == 0 {
            if let Some(next) = hash_list.get(i+1) {
                merkle_paths.insert(
                    Sha256Hash(el.0),
                    Ops::new(vec![Op::Append(next.0.to_vec()), Op::Sha256]) );
            };
        } else {
            merkle_paths.insert(
                Sha256Hash(el.0),
                Ops::new(vec![Op::Prepend(hash_list[i - 1].0.to_vec()), Op::Sha256]) );
        };
    }

    merkle_root_and_paths(&hash_pairs, merkle_paths)
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


#[cfg(test)]
mod tests {
    use merkle::sha256;
    use merkle::sha256_two_input;
    use merkle::make;
    use data_encoding::HEXLOWER;

    #[test]
    fn test_sha256() {
        let empty = Vec::new();
        let a = sha256(&empty[..]);
        let b = HEXLOWER.decode("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".as_bytes());
        assert_eq!(&a.0[..],&b.unwrap()[..]);
    }

    #[test]
    fn test_make() {
        let empty = Vec::new();
        let first_leaf = sha256(&empty[..]);
        let mut digests = Vec::new();
        digests.push(first_leaf.clone());
        let (root, paths) = make(&digests);
        assert_eq!(&root, &first_leaf);    // merkle tree with one element
        println!("---> {:?}", paths);

        let second_leaf = sha256(&empty[..]);
        digests.push(second_leaf.clone());
        let expected_root = sha256_two_input(&first_leaf.0, &second_leaf.0);
        let (root, paths) = make(&digests);
        assert_eq!(root, expected_root );
        println!("---> {:?}", paths);

        let third_leaf = sha256(&empty[..]);
        digests.push(third_leaf.clone());
        let expected_root_3 = sha256_two_input(&expected_root.0, &first_leaf.0);
        let (root, paths) = make(&digests);
        assert_eq!(root, expected_root_3 );
        println!("---> {:?}", paths);

        let fourth_leaf = sha256(&empty[..]);
        digests.push(fourth_leaf.clone());
        let expected_root_4 = sha256_two_input(&expected_root.0, &expected_root.0);
        let (root, paths) = make(&digests);
        assert_eq!(root, expected_root_4 );
        println!("---> {:?}", paths);
    }
}
