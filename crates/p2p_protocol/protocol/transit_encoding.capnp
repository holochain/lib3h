# lib3h In-Transit Encoding Handshake Protocol
# These are all separate structs, rather than having a top-level root/union
# This works because there is a well-defined handshake expectation
# Anything unexpected is an error which should cause a connection drop

@0xdf3e1d96ae657243;

struct Halt {
  # Let's at least try to be nice to people connecting to us
  # Give them some info why we won't accept their connection
  # Note: We may disable Halt responses for production to prevent
  # leaking information, but for protocol testing / debugging
  # it is useful to have the capability.

  reasonCode @0 :ReasonCode;
  # If the error is well known, include the reasonCode, otherwise unspecified

  reasonText @1 :Text;
  # the specific details of the halt reason

  enum ReasonCode {
    unspecified @0;
    badMagic @1;
    badEncoding @2;
    badNetworkId @3;
    kxDecodeFail @4;
    unexpectedSigPubKey @5;
    badSignature @6;
  }
}

struct MsgStep1FromConnect {
  # When opening a connection, you should send this first message

  magic @0 :UInt16 = 0x0000;
  # Protocol Identifier, this should be 0xa86c
  # Note, default above is 0x0000, because otherwise 0xa86c would not be sent

  encoding @1 :Encoding;
  # How should we handle this stream (the remote may choose not to accept)

  networkId @2 :Data;
  # The network identifier we are trying to join
  # If the remote has a different id, it may drop the connection.
  # TODO - let's remove this to not expose networkId
  # We could put networkId in additional data for the KX exchange
  # (or just in the content for better error reporting)

  kxPubKey @3 :Data;
  # Send the remote our key exchange public key

  enum Encoding {
    # Which encoding scheme should we use

    unknown @0;
    # Default, you may get this if someone's using a newer protocol

    openJson @1;
    # Un-encrypted Json encoded

    openPacked @2;
    # Un-encrypted packed Capnproto encoded

    sodiumJson @3;
    # libsodium-encrypted Json encoded

    sodiumPacked @4;
    # libsodium-encrypted packed Capnproto encoded
  }
}

struct MsgStep2FromListen {
  # A remote node has connected to us, and sent MsgConH1
  # We need to either accept or reject their request

  union {
    halt @0 :Halt;
    continue @1 :Continue;
  }

  struct Continue {
    # We are cool with the remote connecting node's request
    # Lets send them the info they need to proceed
    # note: kxSecret should contain an encoded MsgLsnH2Kx Message
    #       if we are an open encoding, it'll just be the data as bytes

    kxPubKey @0 :Data;
    # Send the remote connecting node or key exchange public key

    kxNonce @1: Data;
    # If we are libsodium, kxSecret will be encrypted using kx derivation
    # We need to encrypt with a random nonce, and forward that to the remote
    # This can be empty for open encodings.

    kxSecret @2 :Data;
    # contains the bytes of MsgLsnH2Kx
  }
}

struct MsgStep2FromListenKxEncoded {
  # this message will be encoded into the kxSecret field in MsgLsnH2

  padding @0 :Data;

  sigPubKey @1 :Data;
  # this node's signature public key (transportId or agentId)

  l2cSessionKey @2 :Data;
  # pure entropy listening-to-connecting session key
}

struct MsgStep3FromConnect {
  # this message is also kx encrypted (see MsgLsnH2)

  union {
    halt @0 :Halt;
    continue @1 :Continue;
  }

  struct Continue {
    kxNonce @0 :Data;
    # kx nonce

    kxSecret @1 :Data;
    # kx secrets (see MsgConH3Kx)
  }
}

struct MsgStep3FromConnectKxEncoded {
  # this message will be encoded into the kxSecret field in MsgConH3

  padding @0 :Data;

  sigPubKey @1 :Data;
  # this node's signature public key (transportId or agentId)

  c2lSessionKey @2 :Data;
  # pure entropy connecting-to-listening session key

  c2lSignature @3 :Data;
  # signature of l2cSessionKey proving we own sig priv key
}

struct MsgStep4FromListenEncoded {
  # this message is encoded directly, not wrapped like Kx above
  # this message will use nonce-0 of the l2cSessionKey

  padding @0 :Data;

  l2cSignature @1 :Data;
  # signature of c2lSessionKey proving we own sig priv key
}

struct MsgStep5FromConnectEncoded {
  # this message is encoded directly, not wrapped like Kx above
  # this message will use nonce-0 of the c2lSessionKey
  # when this message is received, we can upgrade to the next protocol

  padding @0 :Data;
}

struct EncodedMessage {
  # We have made it past the handshake sequence
  # We can now start exchanging encoded data using sequential nonces
  # and the session keys.
  # Note: EncodedMessage + Steps 4 and 5 above may use bucketed timestamps
  # in the aead additional data to ensure timelyness of communications.
  # This framing allows us to specify padding if we'd like to normalize
  # the message lengths and/or inject steganography.

  padding @0 :Data;
  content @1 :Data;
}
