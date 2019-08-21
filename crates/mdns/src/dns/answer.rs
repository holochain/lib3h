//! DNS Answer part.

use std::{io::Cursor, default::Default};
#[allow(unused_imports)]
use byteorder::{BigEndian, ByteOrder, ReadBytesExt, WriteBytesExt};
use crate::error::MulticastDnsResult;

/// Response answer
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Unknown(Vec<u8>),
    Data(AnswerSection),
}


/// Answer section of a DNS message packet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnswerSection {
    pub(crate) dn_len: u16,
    pub(crate) domain_name: String,
    // Should be CNAME = 5
    pub(crate) answer_type: u16,
    // = 1 ?
    pub(crate) answer_class: u16,
    pub(crate) ttl: u32,
    pub(crate) data_len: u16,
    pub(crate) data: Vec<Target>,
}

impl Default for AnswerSection {
    fn default() -> Self {
        Self {
            dn_len: 0,
            domain_name: String::default(),
            answer_type: 5,
            answer_class: 1,
            ttl: 255,
            data_len: 0,
            data: Vec::new()
        }
    }
}

impl AnswerSection {
    /// New with default values for ttl(255), type(5) and class(1).
    pub fn new(name: &str, targets: &[Target]) -> Self {
        Self {
            dn_len: name.len() as u16,
            domain_name: name.to_owned(),
            data_len: targets.len() as u16,
            data: targets.to_vec(),
            ..Default::default()
        }
    }

    /// New with Time To Live value specified.
    pub fn new_with_ttl(name: &str, targets: &[Target], ttl: u32) -> Self {
        Self {
            dn_len: name.len() as u16,
            domain_name: name.to_owned(),
            ttl,
            data_len: targets.len() as u16,
            data: targets.to_vec(),
            ..Default::default()
        }
    }

    /// Builds an [`AnswerSection`] from a byte cursor.
    pub fn from_raw(mut cursor: &mut Cursor<&Vec<u8>>) -> MulticastDnsResult<Self> {
        let dn_len = cursor.read_u16::<BigEndian>()?;
        let mut domain_name: Vec<u8> = Vec::with_capacity(dn_len as usize);
        for _ in 0..dn_len {
            domain_name.push(
                cursor.read_u8()?
            );
        }
        let answer_type = cursor.read_u16::<BigEndian>()?;
        let answer_class = cursor.read_u16::<BigEndian>()?;
        let ttl = cursor.read_u32::<BigEndian>()?;
        let data_len = cursor.read_u16::<BigEndian>()?;
        let mut data = Vec::with_capacity(data_len as usize);
        
        for _ in 0..data_len {
            data.push(Target::from_raw(&mut cursor)?);
        }


        Ok(Self {
            dn_len,
            domain_name: String::from(std::str::from_utf8(&domain_name)?),
            answer_type,
            answer_class,
            ttl,
            data_len,
            data,
        })
    }

    pub fn write(&self, mut packet: &mut Vec<u8>) -> MulticastDnsResult<()> {
        packet.write_u16::<BigEndian>(self.dn_len)?;

        for byte in self.domain_name.as_bytes().to_vec() {
            packet.write_u8(byte)?;
        }


        packet.write_u16::<BigEndian>(self.answer_type)?;
        packet.write_u16::<BigEndian>(self.answer_class)?;
        packet.write_u32::<BigEndian>(self.ttl)?;
        packet.write_u16::<BigEndian>(self.data_len)?;

        for target in self.data.iter() {
            target.write(&mut packet)?;
        }



        Ok(())
    }
}


/// Correspond to the URL [`advertised`](https://docs.rs/lib3h_protocol/0.0.10/lib3h_protocol/network_engine/trait.NetworkEngine.html#tymethod.advertise)
/// by the [`NetworkEngine`](https://docs.rs/lib3h_protocol/0.0.10/lib3h_protocol/network_engine/trait.NetworkEngine.html) from [`Lib3h_protocol`](https://crates.io/crates/lib3h_protocol).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub(crate) target_len: u16,
    pub(crate) target: String,
}

impl Target {
    pub fn new(record: &str) -> Self {
        Self {
            target_len: record.len() as u16,
            target: record.to_owned()
        }
    }
    pub fn from_raw(cursor: &mut Cursor<&Vec<u8>>) -> MulticastDnsResult<Self> {
        let target_len = cursor.read_u16::<BigEndian>()?;
        let mut target = Vec::with_capacity(target_len as usize);
        for _ in 0..target_len {
            target.push(cursor.read_u8()?);
        }

        Ok(Self {
            target_len,
            target: String::from(std::str::from_utf8(&target)?),
        })
    }

    pub fn write(&self, packet: &mut Vec<u8>) -> MulticastDnsResult<()> {
        packet.write_u16::<BigEndian>(self.target_len)?;
        for byte in self.target.as_bytes().to_vec() {
            packet.write_u8(byte)?;
        }

        Ok(())
    }
}

#[test]
fn target_io_test() {
    let target = Target::new("wss:/192.168.0.88");

    let mut buffer = Vec::new();
    target.write(&mut buffer).expect("Fail to write target to buffer.");

    let mut cursor = Cursor::new(&buffer);
    let target_from_raw = Target::from_raw(&mut cursor).expect("Fail to deserialize target from byte buffer.");

    assert_eq!(target, target_from_raw);
}

#[test]
fn answer_with_target_test() {
    let name = "holonaute.local.";
    let targets = vec![Target::new("wss:/192.168.0.88")];
    let answer = AnswerSection::new(name, &targets);

    let mut buffer = vec![];
    answer.write(&mut buffer).expect("Fail to write AnswerSection to buffer.");

    let mut cursor = Cursor::new(&buffer);
    let answer_from_raw = AnswerSection::from_raw(&mut cursor).expect("Fail to deserialize AnswerSection from byte buffer.");

    assert_eq!(answer, answer_from_raw);
}
