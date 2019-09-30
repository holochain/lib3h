# lib3h p2p wire protocol
# note, initial handshake will use transit-enc.capnp
# once that is complete, messages from this schema
# will be encoded as negotiated.

@0x859198991b95d3e1;

struct P2pMessage {
  # Main P2P Message struct, anon union determines message type

  union {
    msgError @0 :MsgError;
    # indicates an error
    # if the error is not recoverable, the connection will be closed

    msgHandshake @1 :MsgHandshake;
    # On a new connection, tell the remote node about ourselves.

    msgGspArcRequest @2 :MsgGspArc;
    # Open a gossip sequence with a remote node.

    msgGspArcResponse @3 :MsgGspArc;
    # Second stage of a gossip sequence with a remote node.

    msgGspAspectDataRequest @4 :MsgGspAspectDataRequest;
    # If we have determined a list of aspect-hashes we require
    # from a remote node, request them.

    msgGspAspectDataResponse @5 :MsgGspAspectDataResponse;
    # A remote node has requested aspect data from us, give it to them.

    msgGspAspectBroadcast @6 :MsgGspAspectBroadcast;
    # Fast push new dht data

    msgDirectRequest @7 :MsgDirect;
    # node-to-node message request

    msgDirectResponse @8 :MsgDirect;
    # node-to-node message response

    msgQueryRequest @9 :MsgQuery;
    # dht query request

    msgQueryResponse @10 :MsgQuery;
    # dht query response

    msgPing @11 :MsgPing;
    # see if we are able to message remote node

    msgPong @12 :MsgPong;
    # respond to a ping from a remote node
  }

  # -- top-level Message Types -- #

  struct MsgError {
    errorCode @0 :ErrorCode;
    # code indicating if error is well-known

    errorText @1 :Text;
    # text indicating details of error

    enum ErrorCode {
      unknown @0;
      # default if error is not well-known, or if remote is using a newer proto
    }
  }

  struct MsgHandshake {
    storeArcRadius @0 :UInt32;
    # the storage arc length of the source transport / agent

    queryArcRadius @1 :UInt32;
    # the query arc length of the source transport / agent
  }

  struct MsgGspArc {
    # data for `msgGspArcRequest` and `msgGspArcResponse`

    aspectConstraintArc @0 :AspectConstraintArc;
    # constrain aspectHashList by loc arc

    aspectConstraintTime @1 :AspectConstraintTime;
    # constrain aspectHashList by publish timestamp

    aspectConstraintCount @2 :AspectConstraintCount;
    # constrain aspectHashList by local store count

    aspectHashList @3 :List(AspectHashList);
    # list of aspectHashes associated with entryAddresses
    # that fall within all the above constraints.
  }

  struct MsgGspAspectDataRequest {
    # request a list of aspect hashes

    aspectHashList @0 :List(AspectHashList);
    # the aspect hashes we are requesting
  }

  struct MsgGspAspectDataResponse {
    # respond to an aspectDataRequest with aspect data

    aspectDataList @0 :List(AspectDataList);
    # the aspect data to respond with
  }

  struct MsgGspAspectBroadcast {
    # fast push new published data

    aspectDataList @0 :List(AspectDataList);
    # the aspect data to publish / broadcast
  }

  struct MsgDirect {
    # node-to-node message data

    requestId @0 :Text;
    # requestId for associating requests / responses

    data @1 :Data;
    # the content of the direct message
  }

  struct MsgQuery {
    # dht query message data

    requestId @0 :Text;
    # requestId for associating requests / responses

    entryAddress @1 :Data;
    # the entryAddress being queried

    data @2 :Data;
    # the message content (either request or response)
  }

  struct MsgPing {
    # see if we are able to message remote node

    pingSendEpochMs @0 :UInt64;
    # the epoch ms timestamp the origin node sent the ping
  }

  struct MsgPong {
    # respond to a ping from a remote node

    pingSendEpochMs @0 :UInt64;
    # the epoch ms ping timestamp (copied) from the origin node

    pingReceivedEpochMs @1 :UInt64;
    # the epoch ms timestamp the target node received the ping
  }

  # -- additional data types -- #

  struct AspectHashList {
    entryAddress @0 :Data;
    # when referring to aspect hashes, we need them to be
    # associated with an entryAddress

    aspectHashList @1 :List(Data);
    # the list of aspectHashes associated with the above entryAddress
    # note this list is probably not comprehensive,
    # it may be only those that fall within constraints, or only
    # those that are being requested / responded with / etc.
  }

  struct AspectDataList {
    entryAddress @0 :Data;
    # when asking for aspect hash data, we also want to know the entryAddress

    aspectDataList @1 :List(Data);
    # the aspect data corresponding to the aspect hashes that were requested
  }

  struct AspectConstraintArc {
    arcStart @0 :UInt32;
    # the start location of the rrdht arc (inclusive)

    arcEnd @1 :UInt32;
    # the end location of the rrdht arc (exclusive)
  }

  struct AspectConstraintTime {
    gteEpochMs @0 :UInt64;
    # aspects must have a publish time >= this value
  }

  struct AspectConstraintCount {
    gteLocalCount @0 :UInt64;
    # aspects must have a local index count >= this value
  }
}
