use bitcoin_hashes::Sha256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Sha256,
    Append([u8; 32]),
    Prepend([u8; 32]),
}

fn sha256_leaf(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    Sha256::hash_byte_chunks(&[left, right]).to_byte_array()
}

fn hash_pairs(mut digests: &[[u8; 32]]) -> Vec<[u8; 32]> {
    let mut r = Vec::with_capacity(digests.len() / 2);
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

fn hash_tree(digests: &[[u8; 32]]) -> (Vec<Vec<Op>>, [u8; 32]) {
    assert!(digests.len() > 0);

    let mut prev_level = digests;
    let mut inner_levels = vec![];

    while prev_level.len() > 1 {
        inner_levels.push(hash_pairs(prev_level));
        prev_level = inner_levels.last().expect("FIXME");
        dbg!(prev_level.len());
    }

    let mut levels: Vec<&[[u8; 32]]> = vec![digests];
    levels.extend(inner_levels.iter().map(|v| v.as_slice()));

    let mut r = vec![];
    for i in 0 .. digests.len() {
        eprintln!("for digest i = {}", i);
        let mut steps = vec![];
        for j in 0 .. levels.len() - 1 {
            eprintln!("level j = {}", j);
            match dbg!((i >> j) & 0b1) {
                0 => {
                    if let Some(sibling) = levels[j].get((i >> j) + 1) {
                        steps.push(Op::Append(*sibling));
                    } else {
                        // Odd-numbered hash, duplicated.
                        steps.push(dbg!(Op::Append(levels[j][i >> j])));
                    }
                },
                1 => {
                    steps.push(Op::Prepend(levels[j][(i >> j) - 1]));
                },
                _ => unreachable!(),
            };
            steps.push(Op::Sha256);
        }
        r.push(steps);
    }
    (r, levels.last().unwrap()[0])
}

//    a
//  a   b
// a b c d
// 0 1 2 3

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

    #[test]
    fn test_hash_tree() {
        let (digest_steps, tip) = hash_tree(&[[0; 32]]);
        assert_eq!(digest_steps, vec![vec![]]);
        assert_eq!(tip, [0; 32]);

        let (digest_steps, tip) = hash_tree(&[[0; 32], [1; 32]]);
        assert_eq!(digest_steps,
                   vec![vec![Op::Append([1; 32]), Op::Sha256],
                        vec![Op::Prepend([0; 32]), Op::Sha256]]);
        assert_eq!(tip, [92, 133, 149, 95, 112, 146, 131, 236, 206, 43, 116, 241, 177, 85, 41, 24, 129, 159, 57, 9, 17, 129, 110, 123, 180, 102, 128, 90, 56, 171, 135, 243]);

        let (digest_steps, tip) = hash_tree(&[[0; 32], [1; 32], [2; 32]]);
        assert_eq!(digest_steps,
                   vec![vec![Op::Append([1; 32]), Op::Sha256,
                             Op::Append([248, 59, 51, 43, 228, 230, 165, 164, 177, 197, 106, 175, 109, 181, 38, 87, 218, 73, 94, 20, 152, 112, 5, 125, 133, 144, 171, 157, 122, 97, 103, 173]), Op::Sha256],
                        vec![Op::Prepend([0; 32]), Op::Sha256,
                             Op::Append([248, 59, 51, 43, 228, 230, 165, 164, 177, 197, 106, 175, 109, 181, 38, 87, 218, 73, 94, 20, 152, 112, 5, 125, 133, 144, 171, 157, 122, 97, 103, 173]), Op::Sha256],
                        vec![Op::Append([2; 32]), Op::Sha256,
                             Op::Prepend([92, 133, 149, 95, 112, 146, 131, 236, 206, 43, 116, 241, 177, 85, 41, 24, 129, 159, 57, 9, 17, 129, 110, 123, 180, 102, 128, 90, 56, 171, 135, 243]), Op::Sha256]]);
        assert_eq!(tip, [109, 239, 207, 248, 67, 177, 45, 214, 132, 22, 37, 128, 195, 65, 6, 82, 131, 134, 158, 75, 46, 9, 234, 154, 39, 193, 157, 153, 116, 98, 165, 60]);

        let (digest_steps, tip) = hash_tree(&[[0; 32], [1; 32], [2; 32], [3; 32]]);
        assert_eq!(digest_steps,
                   vec![vec![Op::Append([1; 32]), Op::Sha256,
                             Op::Append([39, 243, 47, 187, 250, 194, 251, 187, 206, 88, 177, 7, 82, 20, 75, 90, 116, 70, 212, 185, 30, 75, 169, 15, 253, 238, 48, 94, 145, 89, 128, 232]), Op::Sha256],
                        vec![Op::Prepend([0; 32]), Op::Sha256,
                             Op::Append([39, 243, 47, 187, 250, 194, 251, 187, 206, 88, 177, 7, 82, 20, 75, 90, 116, 70, 212, 185, 30, 75, 169, 15, 253, 238, 48, 94, 145, 89, 128, 232]), Op::Sha256],
                        vec![Op::Append([3; 32]), Op::Sha256,
                             Op::Prepend([92, 133, 149, 95, 112, 146, 131, 236, 206, 43, 116, 241, 177, 85, 41, 24, 129, 159, 57, 9, 17, 129, 110, 123, 180, 102, 128, 90, 56, 171, 135, 243]), Op::Sha256],
                        vec![Op::Prepend([2; 32]), Op::Sha256,
                             Op::Prepend([92, 133, 149, 95, 112, 146, 131, 236, 206, 43, 116, 241, 177, 85, 41, 24, 129, 159, 57, 9, 17, 129, 110, 123, 180, 102, 128, 90, 56, 171, 135, 243]), Op::Sha256]]);
        assert_eq!(tip, [211, 95, 81, 105, 147, 137, 218, 126, 236, 124, 229, 235, 2, 100, 12, 109, 49, 140, 245, 26, 227, 158, 202, 137, 11, 188, 123, 132, 236, 181, 218, 104]);

        let (digest_steps, tip) = hash_tree(&[[0; 32], [1; 32], [2; 32], [3; 32], [4; 32], [5; 32], [6; 32], [7;32], [8; 32]]);
        assert_eq!(tip, [2, 13, 235, 58, 9, 19, 117, 234, 116, 28, 73, 93, 142, 23, 15, 38, 132, 232, 87, 160, 158, 71, 203, 108, 180, 79, 99, 227, 168, 102, 58, 177]);
        assert_eq!(digest_steps.len(), 9);
    }
}
