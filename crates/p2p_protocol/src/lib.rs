//! Lib3h Protocol definition for inter-node p2p communication.

extern crate capnp;
extern crate serde;
extern crate serde_derive;
extern crate serde_json;

#[allow(dead_code)]
#[allow(clippy::all)]
#[rustfmt::skip]
mod multiplex_capnp;
#[allow(dead_code)]
#[allow(clippy::all)]
#[rustfmt::skip]
mod p2p_capnp;
#[allow(dead_code)]
#[allow(clippy::all)]
#[rustfmt::skip]
mod transit_encoding_capnp;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_can_encode_decode_ping() {
        let mut message = capnp::message::Builder::new_default();
        {
            let mut ping = message
                .init_root::<p2p_capnp::p2p_message::Builder>()
                .init_msg_ping();
            ping.set_ping_send_epoch_ms(42);
        }
        let mut bin = Vec::new();
        capnp::serialize_packed::write_message(&mut bin, &message).unwrap();

        println!("WROTE: {:?}", bin);
        assert_eq!(
            "[16, 4, 80, 1, 1, 1, 11, 16, 1, 1, 42]",
            format!("{:?}", bin),
        );

        let message = capnp::serialize_packed::read_message(
            &mut std::io::Cursor::new(bin),
            capnp::message::ReaderOptions::new(),
        ).unwrap();
        let message = message
            .get_root::<p2p_capnp::p2p_message::Reader>()
            .unwrap();

        match message.which() {
            Ok(p2p_capnp::p2p_message::MsgPing(Ok(ping))) => {
                println!("READ: ping: {:?}", ping.get_ping_send_epoch_ms());
                assert_eq!(42_u64, ping.get_ping_send_epoch_ms());
            }
            _ => panic!("decode fail"),
        }
    }
}
