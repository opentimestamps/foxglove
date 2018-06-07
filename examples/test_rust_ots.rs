extern crate opentimestamps;

use opentimestamps::Timestamp;
use opentimestamps::timestamp::Step;
use opentimestamps::timestamp::StepData;
use opentimestamps::op::Op;
use opentimestamps::ser::Serializer;
use opentimestamps::attestation::Attestation;

fn main() {
    println!("Ciao");
    let v = Vec::new();


    let dummy = Step {
        data: StepData::Attestation( Attestation::Unknown{
            tag: vec![],
            data: vec![],
        }),
        output: vec![],
        next: vec![],
    };

    let s2 = Step {
        data: StepData::Op(Op::Sha256),
        output: vec![],
        next: vec![dummy],
    };


    let s = Step {
        data: StepData::Op(Op::Append(vec![0x00,0x11])),
        output: vec![],
        next: vec![s2],
    };



    let a = Timestamp {
        start_digest:v,
        first_step:s,
    };

    println!("{}",a);

    let writer = vec![];
    let mut ser = Serializer::new(writer);
    a.serialize(&mut ser).unwrap();
    println!("{:?}",ser.into_inner())

}
