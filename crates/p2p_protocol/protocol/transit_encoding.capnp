# lib3h In-Transit Encoding Handshake Protocol
# These are all separate structs, rather than having a top-level root/union
# This works because there is a well-defined handshake expectation
# Anything unexpected is an error which should cause a connection drop

@0xdf3e1d96ae657243;

struct Halt {
  # Let's at least try to be nice to people connecting to us
  # Give them some info why we won't accept their connection

  reasonCode @0 :ReasonCode;
  reasonText @1 :Text;

  enum ReasonCode {
    unspecified @0;
    badMagic @1;
    badEncoding @2;
    badNetworkId @3;
    kxDecodeFail @4;
    unexpectedSigPubKey @5;
    badSignature @6;
    encDecodeFail @7;
  }
}

struct MsgConH1 {
  # When opening a connection, you should send this first message

  magic @0 :UInt16 = 0x0000;
  # Protocol Identifier, this should be 0xa86c
  # Note, default above is 0x0000, because otherwise 0xa86c would not be sent

  encoding @1 :Encoding;
  # How should we handle this stream (the remote may choose not to accept)

  networkId @2 :Data;
  # The network identifier we are trying to join
  # If the remote has a different id, it may drop the connection.

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

struct MsgLsnH2 {
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

struct MsgLsnH2Kx {
  # this message will be encoded into the kxSecret field in MsgLsnH2

  padding @0 :Data;

  sigPubKey @1 :Data;
  # this node's signature public key (transportId or agentId)

  l2cSessionKey @2 :Data;
  # pure entropy listening-to-connecting session key
}

struct MsgConH3 {
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

struct MsgConH3Kx {
  # this message will be encoded into the kxSecret field in MsgConH3

  padding @0 :Data;

  sigPubKey @1 :Data;
  # this node's signature public key (transportId or agentId)

  c2lSessionKey @2 :Data;
  # pure entropy connecting-to-listening session key

  c2lSignature @3 :Data;
  # signature of l2cSessionKey proving we own sig priv key
}

struct MsgLsnH4Enc {
  # this message is encoded directly, not wrapped like Kx above
  # this message will use nonce-0 of the l2cSessionKey

  padding @0 :Data;

  l2cSignature @1 :Data;
  # signature of c2lSessionKey proving we own sig priv key
}

struct MsgConH5Enc {
  # this message is encoded directly, not wrapped like Kx above
  # this message will use nonce-0 of the c2lSessionKey
  # when this message is received, we can upgrade to the next protocol
  # Further messages will be sent raw without a wrapper in this schema

  padding @0 :Data;
}
