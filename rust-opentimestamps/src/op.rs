pub fn sha1(_msg: &[u8]) -> [u8; 20] {
    todo!()
}

pub fn sha256(_msg: &[u8]) -> [u8; 32] {
    todo!()
}

pub fn ripemd160(_msg: &[u8]) -> [u8; 20] {
    todo!()
}

pub enum Op {
    Sha1,
    Sha256,
    Ripemd160,
    Hexlify,
    Append,
    Prepend,
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
