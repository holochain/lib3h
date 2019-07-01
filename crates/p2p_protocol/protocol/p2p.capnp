# lib3h p2p wire protocol
# note, initial handshake will use transit-enc.capnp
# once that is complete, messages from this schema
# will be encoded as negotiated.

@0x859198991b95d3e1;

struct Message {
  # Main P2P Message struct, anon union determines message type

  union {
    msgError @0 :MsgError;
    # indicates an error
    # if the error is not recoverable, the connection will be closed

    msgChannel @1 :MsgChannel;
    # establish a new channel.
    # All other messages require an established channelId.

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
  }

  # -- top-level Message Types -- #

  struct MsgError {
    channelId @0 :UInt32;
    # Use 0xffffffff to indicate an error not related to a specific channel

    errorCode @1 :ErrorCode;
    # code indicating if error is well-known

    errorText @2 :Text;
    # text indicating details of error

    enum ErrorCode {
      unknown @0;
      # default if error is not well-known, or if remote is using a newer proto

      badChannelId @1;
      # usually, a message was sent without first sending `msgChannel`

      badSpaceHash @2;
      # this node is not a part of this spaceHash

      badToId @3;
      # this node does not have a transportId or agentId matching this id

      badFromId @4;
      # this node does not wish to accept messages from this remote id
    }
  }

  struct MsgChannel {
    # this protocol multiplexes between multiple spaceHash / id pairs
    # to avoid repeating this info in every message, we want to establish
    # symbolic `channelId`s.

    channelId @0 :UInt32;
    # the channel id to establish. Must be unique to this communication session.

    spaceHash @1 :Data;
    # the spaceHash to establish this channel for.

    toId @2 :Data;
    # the destination transportId or agentId to establish this channel for

    fromId @3 :Data;
    # the source transportId or agentId to establish this channel for

    fromStoreArc @4 :UInt32;
    # the storage arc length of the source transport / agent

    fromQueryArc @5 :UInt32;
    # the query arc length of the source transport / agent
  }

  struct MsgGspArc {
    # data for `msgGspArcRequest` and `msgGspArcResponse`

    channelId @0 :UInt32;
    # requisit channelId (see `msgChannel`)

    aspectConstraintLoc @1 :AspectConstraintLoc;
    # constrain aspectHashList by loc arc

    aspectConstraintTime @2 :AspectConstraintTime;
    # constrain aspectHashList by publish timestamp

    aspectConstraintCount @3 :AspectConstraintCount;
    # constrain aspectHashList by local store count

    aspectHashList @4 :List(AspectHashList);
    # list of aspectHashes associated with entryAddresses
    # that fall within all the above constraints.
  }

  struct MsgGspAspectDataRequest {
    # request a list of aspect hashes

    channelId @0 :UInt32;
    # requisit channelId (see `msgChannel`)

    aspectHashList @1 :List(AspectHashList);
    # the aspect hashes we are requesting
  }

  struct MsgGspAspectDataResponse {
    # respond to an aspectDataRequest with aspect data

    channelId @0 :UInt32;
    # requisit channelId (see `msgChannel`)

    aspectHashDataList @1 :List(AspectHashDataList);
    # the aspect data to respond with
  }

  struct MsgGspAspectBroadcast {
    # fast push new published data

    channelId @0 :UInt32;
    # requisit channelId (see `msgChannel`)

    aspectHashDataList @1 :List(AspectHashDataList);
    # the aspect data to publish / broadcast
  }

  struct MsgDirect {
    # node-to-node message data

    channelId @0 :UInt32;
    # requisit channelId (see `msgChannel`)

    requestId @1 :Text;
    # requestId for associating requests / responses

    data @2 :Data;
    # the content of the direct message
  }

  struct MsgQuery {
    # dht query message data

    channelId @0 :UInt32;
    # requisit channelId (see `msgChannel`)

    requestId @1 :Text;
    # requestId for associating requests / responses

    entryAddress @2 :Data;
    # the entryAddress being queried

    data @3 :Data;
    # the message content (either request or response)
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

  struct AspectHashDataList {
    entryAddress @0 :Data;
    # when asking for aspect hash data, we also want to know the entryAddress

    aspectHashDataList @1 :List(AspectHashData);
    # the list of pairs of aspectHash / aspectData
  }

  struct AspectHashData {
    aspectHash @0 :Data;
    # the hash of the aspectData

    aspectData @1 :Data;
    # the data associated with the aspectHash
  }

  struct AspectConstraintLoc {
    locArcStart @0 :UInt32;
    # the start location of the rrdht arc (inclusive)

    locArcEnd @1 :UInt32;
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
