use bitcoin_hashes::Sha256;

fn sha256_leaf(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    Sha256::hash_byte_chunks(&[left, right]).to_byte_array()
}

fn hash_pairs(mut digests: &[[u8; 32]]) -> Vec<[u8; 32]> {
    let mut r = vec![];
    loop {
        if let Some((left, rest)) = digests.split_first() {
            if let Some((right, new_digests)) = rest.split_first() {
                digests = new_digests;
                r.push(sha256_leaf(left, right));
            } else {
                digests = &[];
                r.push(sha256_leaf(left, left));
            }
        } else {
            break r;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_pairs() {
        assert_eq!(hash_pairs(&[]), Vec::<[u8; 32]>::new());
        assert_eq!(hash_pairs(&[[0; 32],
                                [1; 32]]),
                   &[[92, 133, 149, 95, 112, 146, 131, 236, 206, 43, 116, 241, 177, 85, 41, 24, 129, 159, 57, 9, 17, 129, 110, 123, 180, 102, 128, 90, 56, 171, 135, 243]]);
        assert_eq!(hash_pairs(&[[0; 32]]),
                   &[[245, 165, 253, 66, 209, 106, 32, 48, 39, 152, 239, 110, 211, 9, 151, 155, 67, 0, 61, 35, 32, 217, 240, 232, 234, 152, 49, 169, 39, 89, 251, 75]]);
    }
}
