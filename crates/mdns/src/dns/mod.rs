//! mDNS message serializer.

// Allow the use of &Vec<u8> for packet type definition, otherwise it will create diffult to track
// errors while using cursor and byteorder.
#![allow(clippy::ptr_arg)]

use super::error::MulticastDnsResult;
#[allow(unused_imports)]
use byteorder::{BigEndian, ByteOrder, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

pub mod answer;
pub mod question;

pub use answer::{AnswerSection, Target};
pub use question::QuerySection;

/// Structure matching a DNS message format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DnsMessage {
    pub(crate) trans_id: u16,
    pub(crate) parameters: u16,
    pub(crate) nb_questions: u16,
    pub(crate) nb_answers: u16,
    pub(crate) nb_authority: u16,
    pub(crate) nb_additional: u16,
    pub(crate) questions: Vec<QuerySection>,
    pub(crate) answers: Vec<AnswerSection>,
}

impl std::default::Default for DnsMessage {
    fn default() -> Self {
        Self {
            trans_id: 0,
            parameters: 0,
            nb_questions: 0,
            nb_answers: 0,
            nb_authority: 0,
            nb_additional: 0,
            questions: Vec::new(),
            answers: Vec::new(),
        }
    }
}

impl DnsMessage {
    pub fn new() -> Self {
        DnsMessage::default()
    }

    pub fn nb_questions(&self) -> u16 {
        self.nb_questions
    }

    pub fn nb_answers(&self) -> u16 {
        self.nb_answers
    }

    pub fn from_raw(packet: &Vec<u8>) -> MulticastDnsResult<Self> {
        let mut dmesg = DnsMessage::new();

        let mut cursor = Cursor::new(packet);

        dmesg.trans_id = cursor.read_u16::<BigEndian>()?;
        dmesg.parameters = cursor.read_u16::<BigEndian>()?;
        dmesg.nb_questions = cursor.read_u16::<BigEndian>()?;
        dmesg.nb_answers = cursor.read_u16::<BigEndian>()?;
        dmesg.nb_authority = cursor.read_u16::<BigEndian>()?;
        dmesg.nb_additional = cursor.read_u16::<BigEndian>()?;

        for _ in 0..dmesg.nb_questions {
            let dn_len = cursor.read_u16::<BigEndian>()?;
            let question = QuerySection::from_raw(dn_len, &mut cursor)?;
            dmesg.questions.push(question);
        }

        for _ in 0..dmesg.nb_answers {
            // let dn_len = cursor.read_u16::<BigEndian>()?;
            let answer = AnswerSection::from_raw(&mut cursor)?;
            dmesg.answers.push(answer);
        }

        Ok(dmesg)
    }

    pub fn to_raw(&self) -> MulticastDnsResult<Vec<u8>> {
        let mut packet = Vec::with_capacity(512);

        packet.write_u16::<BigEndian>(self.trans_id)?;
        packet.write_u16::<BigEndian>(self.parameters)?;
        packet.write_u16::<BigEndian>(self.nb_questions)?;
        packet.write_u16::<BigEndian>(self.nb_answers)?;
        packet.write_u16::<BigEndian>(self.nb_authority)?;
        packet.write_u16::<BigEndian>(self.nb_additional)?;

        for question in self.questions.iter() {
            question.write(&mut packet)?;
        }

        for answer in self.answers.iter() {
            answer.write(&mut packet)?;
        }

        Ok(packet)
    }
}

#[test]
fn dns_message_question_test() {
    let question = QuerySection::new("holonaute.local.");
    let dmesg = DnsMessage {
        nb_questions: 1,
        questions: vec![question],
        ..Default::default()
    };

    println!("dmesg Query = {:#?}", &dmesg);

    let packet = dmesg
        .to_raw()
        .expect("Fail to convert DnsMessage to bytes.");
    println!("packet = {:?}", &packet);

    let dmesg_from_raw =
        DnsMessage::from_raw(&packet).expect("Fail to deserialize DnsMessage from bytes");

    assert_eq!(dmesg, dmesg_from_raw);
}

#[test]
fn dns_message_answer_test() {
    let targets = vec![Target::new("wss://192.168.0.88")];
    let answer = AnswerSection::new("holonaute.local.", &targets);

    let dmesg = DnsMessage {
        nb_answers: 1,
        answers: vec![answer],
        ..Default::default()
    };

    println!("dmesg Response = {:#?}", &dmesg);

    let packet = dmesg
        .to_raw()
        .expect("Fail to convert DnsMessage to bytes.");
    println!("packet = {:?}", &packet);

    let dmesg_from_raw =
        DnsMessage::from_raw(&packet).expect("Fail to deserialize DnsMessage from bytes");

    assert_eq!(dmesg, dmesg_from_raw);
}

#[test]
fn dns_message_multiple_question_and_answer_test() {
    let questions = vec![
        QuerySection::new("holonaute.local."),
        QuerySection::new("mistral.local."),
    ];

    let answers = vec![
        AnswerSection::new("holonaute.local.", &[Target::new("wss://192.168.0.88")]),
        AnswerSection::new("holonaute.local.", &[Target::new("wss://192.168.0.89")]),
        AnswerSection::new("mistral.local.", &[Target::new("wss://192.168.0.77")]),
        AnswerSection::new("mistral.local.", &[Target::new("wss://192.168.0.78")]),
    ];

    let dmesg = DnsMessage {
        nb_questions: questions.len() as u16,
        nb_answers: answers.len() as u16,
        questions,
        answers,
        ..Default::default()
    };

    println!("dmesg Response = {:#?}", &dmesg);
    let packet = dmesg
        .to_raw()
        .expect("Fail to convert DnsMessage to bytes.");
    println!("packet = {:?}", &packet);

    let dmesg_from_raw =
        DnsMessage::from_raw(&packet).expect("Fail to deserialize DnsMessage from bytes");

    assert_eq!(dmesg, dmesg_from_raw);
}
