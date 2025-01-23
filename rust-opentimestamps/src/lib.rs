pub mod op;

/*
/// The output of an opcode.
pub struct OpOutput(Box<[u8]>);

pub struct LinearTimestampBuilder {
    ops: Vec<u8>,
    output: OpOutput,
}

impl From<OpOutput> for LinearTimestampBuilder {
    fn from(output: OpOutput) -> Self {
        Self {
            ops: vec![],
            output,
        }
    }
}

impl LinearTimestampBuilder {
    pub fn push_hash_op(&mut self, op: HashOp) {
        todo!()
    }

    pub fn try_push_op(&mut self, op: Op) -> Result<(), OpError> {
        todo!()
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
    }
}
