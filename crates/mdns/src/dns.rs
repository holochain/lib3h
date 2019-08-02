//! Encoding utilities for dns packets

use super::error::{MulticastDnsError, MulticastDnsResult};
use byteorder::{BigEndian, ByteOrder, ReadBytesExt, WriteBytesExt};
use std::io::Read;

/// SRV record within a question
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SrvDataQ {
    pub name: Vec<u8>,
}

/// SRV record within an answer
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SrvDataA {
    pub name: Vec<u8>,
    pub ttl_seconds: u32,
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: Vec<u8>,
}

/// query question
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Question {
    Unknown,
    Srv(SrvDataQ),
}

/// response answer
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Unknown(Vec<u8>),
    Srv(SrvDataA),
}

/// dns packet
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    pub id: u16,
    pub is_query: bool,
    pub questions: Vec<Question>,
    pub answers: Vec<Answer>,
}

impl Packet {
    /// create a blank dns packet
    /// fill it in with questions or
    /// set is_query to `false` and fill with answers
    pub fn new() -> Self {
        Packet {
            id: 0,
            is_query: true,
            questions: vec![],
            answers: vec![],
        }
    }

    /// parse a dns packet into a Packet struct
    pub fn with_raw(packet: &[u8]) -> MulticastDnsResult<Self> {
        let mut cursor = std::io::Cursor::new(packet);
        let mut out = Packet::new();

        out.id = cursor.read_u16::<BigEndian>()?;
        out.is_query = cursor.read_u16::<BigEndian>()? == 0;

        let question_count = cursor.read_u16::<BigEndian>()?;
        let answer_count = cursor.read_u16::<BigEndian>()?;

        // nameserver count
        cursor.read_u16::<BigEndian>()?;

        // additional count
        cursor.read_u16::<BigEndian>()?;

        for _ in 0..question_count {
            let svc_name = read_qname(&mut cursor)?;
            let kind = cursor.read_u16::<BigEndian>()?;
            let _class = cursor.read_u16::<BigEndian>()?;

            if kind == 33 {
                out.questions
                    .push(Question::Srv(SrvDataQ { name: svc_name }));
            } else {
                out.questions.push(Question::Unknown);
            }
        }

        for _ in 0..answer_count {
            let svc_name = read_qname(&mut cursor)?;
            let kind = cursor.read_u16::<BigEndian>()?;
            let _class = cursor.read_u16::<BigEndian>()?;
            let ttl_seconds = cursor.read_u32::<BigEndian>()?;

            let enc_size = cursor.read_u16::<BigEndian>()? as usize;

            if kind == 33 {
                let priority = cursor.read_u16::<BigEndian>()?;
                let weight = cursor.read_u16::<BigEndian>()?;
                let port = cursor.read_u16::<BigEndian>()?;
                let target = read_qname(&mut cursor)?;
                out.answers.push(Answer::Srv(SrvDataA {
                    name: svc_name,
                    ttl_seconds,
                    priority,
                    weight,
                    port,
                    target,
                }));
            } else {
                let mut raw = vec![0; enc_size];
                cursor.read_exact(&mut raw)?;
                out.answers.push(Answer::Unknown(raw));
            }
        }

        Ok(out)
    }

    /// encode a Packet struct into a raw dns packet
    pub fn to_raw(&self) -> MulticastDnsResult<Vec<u8>> {
        let mut out = Vec::with_capacity(500);

        // id
        out.write_u16::<BigEndian>(self.id)?;

        // query
        if self.is_query {
            out.write_u16::<BigEndian>(0)?;
        } else {
            out.write_u16::<BigEndian>(0x8400)?;
        }

        // question count
        out.write_u16::<BigEndian>(self.questions.len() as u16)?;

        // answer count
        out.write_u16::<BigEndian>(self.answers.len() as u16)?;

        // unimplemented nameserver count
        out.write_u16::<BigEndian>(0)?;

        // unimplemented additional count
        out.write_u16::<BigEndian>(0)?;

        // add questions
        for q in self.questions.iter() {
            match q {
                Question::Unknown => {
                    return Err(MulticastDnsError::Other(
                        "unknown question type".to_string(),
                    ));
                }
                Question::Srv(q) => {
                    write_qname(&mut out, &q.name)?;

                    // type SRV
                    out.write_u16::<BigEndian>(33)?;

                    // class IN (prefer broadcast)
                    out.write_u16::<BigEndian>(1)?;
                    // class IN (prefer unicast)
                    //out.write_u16::<BigEndian>(1 | 0x8000)?;
                    // class Any (prefer broadcast)
                    //out.write_u16::<BigEndian>(255)?;
                }
            }
        }

        // add answers
        for a in self.answers.iter() {
            match a {
                Answer::Unknown(_) => {
                    return Err(MulticastDnsError::Other(
                        "unknown question type".to_string(),
                    ));
                }
                Answer::Srv(a) => {
                    write_qname(&mut out, &a.name)?;

                    // type SRV
                    out.write_u16::<BigEndian>(33)?;

                    // class IN (prefer broadcast)
                    out.write_u16::<BigEndian>(1)?;
                    // class IN (prefer unicast)
                    //out.write_u16::<BigEndian>(1 | 0x8000)?;
                    // class Any (prefer broadcast)
                    //out.write_u16::<BigEndian>(255)?;

                    // ttl
                    out.write_u32::<BigEndian>(a.ttl_seconds)?;

                    // srv len (will get set after write_qname)
                    let len_offset = out.len();
                    out.write_u16::<BigEndian>(0)?;

                    // priority
                    out.write_u16::<BigEndian>(a.priority)?;

                    // weight
                    out.write_u16::<BigEndian>(a.weight)?;

                    // port
                    out.write_u16::<BigEndian>(a.port)?;

                    // target
                    let len = write_qname(&mut out, &a.target)?;

                    BigEndian::write_u16(&mut out[len_offset..len_offset + 2], len + 6);
                }
            }
        }

        Ok(out)
    }
}

/// write a dot-notation dns name into bytecode parts
fn write_qname<T: byteorder::WriteBytesExt>(out: &mut T, data: &[u8]) -> MulticastDnsResult<u16> {
    let mut len = 0;

    for part in data.split(|&c| c == b'.') {
        out.write_u8(part.len() as u8)?;
        len += 1;
        for c in part.iter() {
            out.write_u8(*c)?;
            len += 1;
        }
    }

    out.write_u8(0)?;
    len += 1;

    Ok(len)
}

/// read raw dns bytecode part name into dot-notation Vec<u8>
fn read_qname<T: byteorder::ReadBytesExt>(read: &mut T) -> MulticastDnsResult<Vec<u8>> {
    let mut out = Vec::with_capacity(500);

    loop {
        let len = read.read_u8()? as usize;
        if len == 0 {
            break;
        }

        if out.len() > 0 {
            out.push(46);
        }

        let olen = out.len();
        out.resize(olen + len, 0);
        read.read_exact(&mut out[olen..olen + len])?;
    }

    Ok(out)
}

#[derive(Debug, Clone)]
pub struct Record {
    /// Hostname of our neighbor
    hostname: String,
    /// IP address in the lan
    ip: String,
}

impl Record {
    /// Create a new record respecting the mDNS
    /// [naming convention](https://tools.ietf.org/html/rfc6762#section-3)
    /// of the form "single-dns-label.local." with value ending with `.local.`
    pub fn new(name: &str, ip: &str) -> Self {
        let hostname = if name.ends_with(".local.") {
            name.to_string()
        } else {
            format!("{}.local.", name)
        };

        Record {
            hostname,
            ip: ip.to_string(),
        }
    }

    /// Returns a reference to the IP address of a neighbor in the LAN.
    pub fn ip(&self) -> &str {
        &self.ip
    }

    /// Returns a reference to the hostname of a neighbor in the LAN.
    pub fn hostname(&self) -> &str {
        &self.hostname
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_srv_q() {
        let mut packet = Packet::new();
        packet.id = 0xbdbd;
        packet.is_query = true;
        packet.questions.push(Question::Srv(SrvDataQ {
            name: b"svc.name.test".to_vec(),
        }));
        let raw = packet.to_raw().unwrap();
        assert_eq!(
            &format!("{:?}", raw),
            "[189, 189, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 3, 115, 118, 99, 4, 110, 97, 109, 101, 4, 116, 101, 115, 116, 0, 0, 33, 0, 1]"
        );
        assert_eq!(packet, Packet::with_raw(&raw).unwrap());
    }

    #[test]
    fn it_should_srv_a() {
        let mut packet = Packet::new();
        packet.id = 0xbdbd;
        packet.is_query = false;
        packet.answers.push(Answer::Srv(SrvDataA {
            name: b"svc.name.test".to_vec(),
            ttl_seconds: 0x12345678,
            priority: 0x2222,
            weight: 0x3333,
            port: 0x4444,
            target: b"svc.name.test".to_vec(),
        }));
        let raw = packet.to_raw().unwrap();
        assert_eq!(
            &format!("{:?}", raw),
            "[189, 189, 132, 0, 0, 0, 0, 1, 0, 0, 0, 0, 3, 115, 118, 99, 4, 110, 97, 109, 101, 4, 116, 101, 115, 116, 0, 0, 33, 0, 1, 18, 52, 86, 120, 0, 21, 34, 34, 51, 51, 68, 68, 3, 115, 118, 99, 4, 110, 97, 109, 101, 4, 116, 101, 115, 116, 0]"
        );
        assert_eq!(packet, Packet::with_raw(&raw).unwrap());
    }
}
