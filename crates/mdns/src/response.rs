//! MDNS record data-types

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    pub questions: Vec<Question>,
    pub answers: Vec<Answer>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Question {
    pub name: String,
    pub class: dns_parser::QueryClass,
    pub kind: dns_parser::QueryType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Answer {
    pub name: String,
    pub class: dns_parser::Class,
    pub ttl: u32,
    pub kind: RecordKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordKind {
    Srv {
        priority: u16,
        weight: u16,
        port: u16,
        target: String,
    },
    Unknown,
}

impl Response {
    pub fn from_packet(packet: &dns_parser::Packet) -> Self {
        Response {
            questions: packet.questions.iter().map(Question::from_packet).collect(),
            answers: packet.answers.iter().map(Answer::from_packet).collect(),
        }
    }
}

impl Question {
    pub fn from_packet(packet: &dns_parser::Question) -> Self {
        Question {
            name: packet.qname.to_string(),
            class: packet.qclass,
            kind: packet.qtype,
        }
    }
}

impl Answer {
    pub fn from_packet(packet: &dns_parser::ResourceRecord) -> Self {
        Answer {
            name: packet.name.to_string(),
            class: packet.cls,
            ttl: packet.ttl,
            kind: RecordKind::from_packet(&packet.data),
        }
    }
}

impl RecordKind {
    pub fn from_packet(packet: &dns_parser::RData) -> Self {
        match *packet {
            dns_parser::RData::SRV(dns_parser::rdata::srv::Record {
                priority,
                weight,
                port,
                ref target,
            }) => RecordKind::Srv {
                priority,
                weight,
                port,
                target: target.to_string(),
            },
            _ => RecordKind::Unknown,
        }
    }
}
