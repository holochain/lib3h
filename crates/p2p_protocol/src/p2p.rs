use crate::{error::P2pResult, p2p_capnp};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MsgPing {
    pub ping_send_epoch_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct MsgPong {
    pub ping_send_epoch_ms: u64,
    pub ping_received_epoch_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub enum P2pMessage {
    MsgPing(MsgPing),
    MsgPong(MsgPong),
}

impl P2pMessage {
    pub fn from_bytes(bytes: Vec<u8>) -> P2pResult<Self> {
        let message = capnp::serialize_packed::read_message(
            &mut std::io::Cursor::new(bytes),
            capnp::message::ReaderOptions::new(),
        )?;

        let message = message
            .get_root::<p2p_capnp::p2p_message::Reader>()
            .unwrap();

        match message.which() {
            Ok(p2p_capnp::p2p_message::MsgPing(Ok(ping))) => Ok(P2pMessage::MsgPing(MsgPing {
                ping_send_epoch_ms: ping.get_ping_send_epoch_ms(),
            })),
            Ok(p2p_capnp::p2p_message::MsgPong(Ok(pong))) => Ok(P2pMessage::MsgPong(MsgPong {
                ping_send_epoch_ms: pong.get_ping_send_epoch_ms(),
                ping_received_epoch_ms: pong.get_ping_received_epoch_ms(),
            })),
            _ => Err("failed to decode".into()),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut message = capnp::message::Builder::new_default();
        {
            match self {
                P2pMessage::MsgPing(ping) => {
                    let mut message = message
                        .init_root::<p2p_capnp::p2p_message::Builder>()
                        .init_msg_ping();

                    message.set_ping_send_epoch_ms(ping.ping_send_epoch_ms);
                }
                P2pMessage::MsgPong(pong) => {
                    let mut message = message
                        .init_root::<p2p_capnp::p2p_message::Builder>()
                        .init_msg_pong();

                    message.set_ping_send_epoch_ms(pong.ping_send_epoch_ms);
                    message.set_ping_received_epoch_ms(pong.ping_received_epoch_ms);
                }
            }
        }
        let mut bytes = Vec::new();
        capnp::serialize_packed::write_message(&mut bytes, &message).unwrap();
        bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_encode_decode_ping() {
        let message = P2pMessage::MsgPing(MsgPing {
            ping_send_epoch_ms: 42,
        });

        let bytes = message.into_bytes();

        assert_eq!(
            "[16, 4, 80, 1, 1, 1, 11, 16, 1, 1, 42]",
            format!("{:?}", bytes),
        );

        match P2pMessage::from_bytes(bytes).unwrap() {
            P2pMessage::MsgPing(ping) => {
                assert_eq!(42_u64, ping.ping_send_epoch_ms);
            }
            _ => panic!("unexpected msg type"),
        }
    }

    #[test]
    fn it_can_encode_decode_pong() {
        let message = P2pMessage::MsgPong(MsgPong {
            ping_send_epoch_ms: 42,
            ping_received_epoch_ms: 99,
        });

        let bytes = message.into_bytes();

        assert_eq!(
            "[16, 5, 80, 1, 1, 1, 12, 16, 2, 1, 42, 1, 99]",
            format!("{:?}", bytes),
        );

        match P2pMessage::from_bytes(bytes).unwrap() {
            P2pMessage::MsgPong(pong) => {
                assert_eq!(42_u64, pong.ping_send_epoch_ms);
                assert_eq!(99_u64, pong.ping_received_epoch_ms);
            }
            _ => panic!("unexpected msg type"),
        }
    }
}
