# For Holochain, the app-level connections use DirectMessage for node2node
# communications between app developer code.
# At the transport-level, once a connection is established between two nodes,
# We use DirectMessage within lib3h to set up multiplexing channels,
# and negotiate relay services, etc.

@0xdd5a14cf7a6734dc;

struct MultiplexMessage {
  # Transport Direct Message Protocol Schema

  union {
    msgError @0 :MsgError;
    # indicates an error
    # if the error is not recoverable, the connection will be closed

    msgChannelCreate @1 :MsgChannelCreate;
    # establish a new multiplexing channel.
    # All other messages require an established channelId.
    # Unless a relay contract has been negotiated, the to_agents
    # must exist on the remote node.

    msgChannelClose @2 :UInt32;
    # close a previously opened channel

    msgChannelMessage @3 :MsgChannelMessage;
    # once we've created a multiplexing channel, we need to be able to message

    msgRelayRequest @4 :Void;
    # send to request the remote node act as a relay

    msgRelayAccept @5 :Void;
    # if the remote node accepts relay duty, they'll send this, otherwise msgError
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

  struct MsgChannelCreate {
    # this protocol multiplexes between multiple spaceHash / id pairs
    # to avoid repeating this info in every message, we want to establish
    # symbolic `channelId`s.

    channelId @0 :UInt32;
    # the channel id to establish. Must be unique to this communication session.

    spaceHash @1 :Data;
    # the spaceHash to establish this channel for.

    toId @2 :Data;
    # the destination agentId to establish this channel for

    fromId @3 :Data;
    # the source agentId to establish this channel for
  }

  struct MsgChannelMessage {
    # a message associated with a previously specified channel

    channelId @0 :UInt32;
    # the previously created channel (see msgChannelCreate)

    content @1 :Data;
    # the content of the message
  }
}
