
use opentimestamps::Timestamp;
use opentimestamps::op::Op;
use opentimestamps::timestamp::Step;
use opentimestamps::timestamp::StepData;
use opentimestamps::attestation::Attestation;
use opentimestamps::ser::Serializer;
use std::collections::HashMap;
use merkle::Sha256Hash;
use std::fmt::Formatter;
use std::fmt;
use data_encoding::HEXLOWER;

#[derive(Debug, Clone)]
pub struct Ops (
    Vec<Op>
);

#[derive(Debug, Clone)]
pub struct LinearTimestamp {
    pub initial_msg: Vec<u8>,
    pub ops: Ops,
}

pub type MerklePaths = HashMap<Sha256Hash, Ops>;



impl fmt::Display for LinearTimestamp {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{} ", HEXLOWER.encode(&self.initial_msg) ).unwrap();
        write!(f, "{} ", &self.ops )
    }
}


impl fmt::Display for Ops {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        for op in &self.0 {
            write!(f, "{} ", op).unwrap();
        }
        Ok(())
    }
}

impl Default for Ops {
    fn default() -> Self {
        Ops(vec![])
    }
}
impl Ops {

    pub fn new(ops : Vec<Op>) -> Self {
        Ops(ops)
    }

    pub fn push(&mut self, op : Op) -> &mut Self {
        self.0.push(op);
        self
    }

    pub fn extend(&mut self, ops : Vec<Op>) -> &mut Self {
        self.0.extend(ops);
        self
    }

    pub fn execute(&self, initial_msg : Vec<u8>) -> Vec<u8> {
        let mut current = initial_msg;
        for op in self.0.iter() {
            current = op.execute(&current);
        }

        current
    }

    pub fn serialize(&self) -> Vec<u8> {
        if self.0.is_empty() {
            return vec![]
        }

        // opentimestamps lib cannot serialize without a final attestation, adding a dummy one
        let dummy_attestation = Step {
            data: StepData::Attestation( Attestation::Unknown{
                tag: vec![],
                data: vec![],
            }),
            output: vec![],
            next: vec![],
        };

        let last = self.0.last().unwrap();
        let mut last_step = Step {
            data: StepData::Op(last.clone()),
            output: vec![],
            next: vec![dummy_attestation],
        };

        for op in self.0.iter().rev().skip(1) {
            let s = Step {
                data: StepData::Op(op.clone()),
                output: vec![],
                next: vec![last_step],
            };
            last_step = s;
        }

        let a = Timestamp {
            start_digest: vec![],
            first_step: last_step,
        };

        let writer = vec![];
        let mut ser = Serializer::new(writer);
        a.serialize(&mut ser).unwrap();
        let mut vec = ser.into_inner();

        // remove the last two bytes of the dummy serialization
        vec.pop();
        vec.pop();

        vec
    }
}


impl LinearTimestamp {
    pub fn new(initial_msg : Vec<u8>) -> Self {
        LinearTimestamp {
            initial_msg,
            ops: Ops::default(),
        }
    }

    pub fn push(&mut self, op : Op) -> &mut Self {
        self.ops.push(op);
        self
    }

    pub fn extend(&mut self, ops : Ops) -> &mut Self {
        self.ops.extend(ops.0);
        self
    }

    pub fn execute(&self) -> Vec<u8> {
        self.ops.execute(self.initial_msg.clone())
    }
}

#[cfg(test)]
mod tests {
    use data_encoding::HEXLOWER;
    use timestamp::Ops;
    use timestamp::LinearTimestamp;
    use opentimestamps::op::Op;

    #[test]
    fn test_ops_serialization() {
        let mut linear = Ops::default();
        linear
            .push(Op::Append(vec![0x00,0x11]) )
            .push(Op::Sha256)
            .push( Op::Prepend(vec![0x05]));

        assert_eq!("f002001108f10105", HEXLOWER.encode( &linear.serialize() ))

    }

    #[test]
    fn test_timestamp_execute() {
        let mut t = LinearTimestamp::new(vec![]);
        t.push(Op::Sha256);
        let result = t.execute();
        assert_eq!("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855", HEXLOWER.encode( &result));

        t.push(Op::Append(vec![0x00]));
        t.push(Op::Prepend(vec![0x00]));
        let result = t.execute();
        assert_eq!("00e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b85500", HEXLOWER.encode( &result));

    }


}
