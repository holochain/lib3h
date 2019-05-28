//! Encoding utilities for dns packets

use super::error::{MulticastDnsError, MulticastDnsResult};
use byteorder::{ByteOrder, BigEndian, WriteBytesExt, ReadBytesExt};
use std::io::Read;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SvcDataQ {
    pub name: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SvcDataA {
    pub name: Vec<u8>,
    pub ttl_seconds: u32,
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Question {
    Unknown(Vec<u8>),
    Svc(SvcDataQ),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Unknown(Vec<u8>),
    Svc(SvcDataA),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Packet {
    pub id: u16,
    pub is_query: bool,
    pub questions: Vec<Question>,
    pub answers: Vec<Answer>,
}

impl Packet {
    pub fn new() -> Self {
        Packet {
            id: 0,
            is_query: true,
            questions: vec![],
            answers: vec![],
        }
    }

    pub fn with_raw(packet: &[u8]) -> MulticastDnsResult<Self> {
        let mut read = std::io::Cursor::new(packet);
        let mut out = Packet::new();

        out.id = read.read_u16::<BigEndian>()?;
        out.is_query = read.read_u16::<BigEndian>()? == 0;

        let question_count = read.read_u16::<BigEndian>()?;
        let answer_count = read.read_u16::<BigEndian>()?;

        // nameserver count
        read.read_u16::<BigEndian>()?;

        // additional count
        read.read_u16::<BigEndian>()?;

        for _ in 0..question_count {
            let svc_name = read_qname(&mut read)?;
            let kind = read.read_u16::<BigEndian>()?;
            let _class = read.read_u16::<BigEndian>()?;

            if kind == 33 {
                out.questions.push(Question::Svc(SvcDataQ {
                    name: svc_name,
                }));
            } else {
                out.questions.push(Question::Unknown(vec![]));
            }
        }

        for _ in 0..answer_count {
            let svc_name = read_qname(&mut read)?;
            let kind = read.read_u16::<BigEndian>()?;
            let _class = read.read_u16::<BigEndian>()?;
            let ttl_seconds = read.read_u32::<BigEndian>()?;

            let enc_size = read.read_u16::<BigEndian>()? as usize;

            if kind == 33 {
                let priority = read.read_u16::<BigEndian>()?;
                let weight = read.read_u16::<BigEndian>()?;
                let port = read.read_u16::<BigEndian>()?;
                let target = read_qname(&mut read)?;
                out.answers.push(Answer::Svc(SvcDataA {
                    name: svc_name,
                    ttl_seconds,
                    priority,
                    weight,
                    port,
                    target,
                }));
            } else {
                let mut raw = Vec::with_capacity(enc_size);
                raw.resize(enc_size, 0);
                read.read_exact(&mut raw)?;
                out.answers.push(Answer::Unknown(raw));
            }
        }

        Ok(out)
    }

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
                Question::Unknown(_) => {
                    return Err(MulticastDnsError::Generic("unknown question type".to_string()));
                },
                Question::Svc(q) => {
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
                    return Err(MulticastDnsError::Generic("unknown question type".to_string()));
                },
                Answer::Svc(a) => {
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
                    out.write_u16::<BigEndian>(0)?;

                    // weight
                    out.write_u16::<BigEndian>(0)?;

                    // port
                    out.write_u16::<BigEndian>(0)?;

                    // target
                    let len = write_qname(&mut out, b"wss://bla")?;

                    BigEndian::write_u16(&mut out[len_offset..len_offset+2], len + 6);
                }
            }
        }

        Ok(out)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_should_srv_q() {
        let mut packet = Packet::new();
        packet.id = 0xbdbd;
        packet.is_query = true;
        packet.questions.push(Question::Svc(SvcDataQ {
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
        packet.answers.push(Answer::Svc(SvcDataA {
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
            "[189, 189, 132, 0, 0, 0, 0, 1, 0, 0, 0, 0, 3, 115, 118, 99, 4, 110, 97, 109, 101, 4, 116, 101, 115, 116, 0, 0, 33, 0, 1, 18, 52, 86, 120, 0, 17, 0, 0, 0, 0, 0, 0, 9, 119, 115, 115, 58, 47, 47, 98, 108, 97, 0]"
        );
        assert_eq!(packet, Packet::with_raw(&raw).unwrap());
    }

    /*
    #[test]
    fn it_should_gen_and_parse() {
        let packet = Packet::new();

        let raw = packet.to_raw().unwrap();
        println!("generated: {:?}", raw);
        assert_eq!(packet, Packet::with_raw(&raw));
    }
    */
}
