//! DNS Question part.

use crate::error::MulticastDnsResult;
#[allow(unused_imports)]
use byteorder::{BigEndian, ByteOrder, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

/// Query question
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Question {
    Unknown(Vec<u8>),
    Data(QuerySection),
}

/// Query section of a DNS message packet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuerySection {
    pub(crate) dn_len: u16,
    pub(crate) domain_name: String,
    /// Not used at the moment
    pub(crate) query_type: u16,
    /// IN(1) fro the Internet.
    pub(crate) query_class: u16,
}

impl QuerySection {
    pub fn new(name: &str) -> Self {
        Self {
            dn_len: name.len() as u16,
            domain_name: name.to_owned(),
            query_type: 0,
            query_class: 1,
        }
    }
    pub fn from_raw(dn_len: u16, cursor: &mut Cursor<&Vec<u8>>) -> MulticastDnsResult<Self> {
        let mut domain_name: Vec<u8> = Vec::with_capacity(dn_len as usize);
        for _ in 0..dn_len {
            domain_name.push(cursor.read_u8()?);
        }

        Ok(Self {
            dn_len,
            domain_name: String::from(std::str::from_utf8(&domain_name)?),
            query_type: cursor.read_u16::<BigEndian>()?,
            query_class: cursor.read_u16::<BigEndian>()?,
        })
    }

    pub fn write(&self, packet: &mut Vec<u8>) -> MulticastDnsResult<()> {
        packet.write_u16::<BigEndian>(self.dn_len)?;

        for byte in self.domain_name.as_bytes().to_vec() {
            packet.write_u8(byte)?;
        }

        packet.write_u16::<BigEndian>(self.query_type)?;
        packet.write_u16::<BigEndian>(self.query_class)?;

        Ok(())
    }
}
