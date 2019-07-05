# Lib3h Multiplex Protocol

```sequence
@startuml
note right of Alice: Alice has established a\ntransport connection with Bob\nand now would like to layer\non an app connection.
Alice -> Bob: msgChannelCreate
note right of Bob: Bob holds agency over\nthe requested spaceHash/\ntoId combination, so he\nregisters the channelId.
note right of Alice: Alice wants to send\nan app message
Alice -> Bob: msgChannelMessage
note right of Bob: Bob matches up the channelId\nand knows the associated\nspaceHash and agentIds.
note right of Alice: Alice now would like to\nuse Bob as a relay.
Alice -> Bob: msgRelayRequest
Bob -> Alice: msgRelayAccept
note right of Alice: Now, Alice can open channels\nthat point to agents\nother than Bob.
Alice -> Alice: PeerDiscovery@Bob
Charlie -> Bob: sends message for Alice
note right of Bob: Bob now has to open\na channel pointing to Alice,\nbut coming from Charlie
Bob -> Alice: msgChannelCreate
Bob -> Alice: Forwards Charlie's message
@enduml
```

http://www.plantuml.com/plantuml

```
     ┌─────┐                      ┌───┐                  ┌───────┐    
     │Alice│                      │Bob│                  │Charlie│    
     └──┬──┘                      └─┬─┘                  └───┬───┘    
        │ ╔═════════════════════════╧═════╗                  │        
        │ ║Alice has established a       ░║                  │        
        │ ║transport connection with Bob  ║                  │        
        │ ║and now would like to layer    ║                  │        
        │ ║on an app connection.          ║                  │        
        │ ╚═════════════════════════╤═════╝                  │        
        │     msgChannelCreate      │                        │        
        │──────────────────────────>│                        │        
        │                           │                        │        
        │                           │ ╔══════════════════════╧═══╗    
        │                           │ ║Bob holds agency over    ░║    
        │                           │ ║the requested spaceHash/  ║    
        │                           │ ║toId combination, so he   ║    
        │                           │ ║registers the channelId.  ║    
        │                           │ ╚══════════════════════╤═══╝    
        │ ╔═════════════════════╗   │                        │        
        │ ║Alice wants to send ░║   │                        │        
        │ ║an app message       ║   │                        │        
        │ ╚═════════════════════╝   │                        │        
        │    msgChannelMessage      │                        │        
        │──────────────────────────>│                        │        
        │                           │                        │        
        │                           │ ╔══════════════════════╧═══════╗
        │                           │ ║Bob matches up the channelId ░║
        │                           │ ║and knows the associated      ║
        │                           │ ║spaceHash and agentIds.       ║
        │                           │ ╚══════════════════════╤═══════╝
        │ ╔═════════════════════════╗                        │        
        │ ║Alice now would like to ░║                        │        
        │ ║use Bob as a relay.      ║                        │        
        │ ╚═════════════════════════╝                        │        
        │     msgRelayRequest       │                        │        
        │──────────────────────────>│                        │        
        │                           │                        │        
        │      msgRelayAccept       │                        │        
        │<──────────────────────────│                        │        
        │                           │                        │        
        │ ╔═════════════════════════╧════╗                   │        
        │ ║Now, Alice can open channels ░║                   │        
        │ ║that point to agents          ║                   │        
        │ ║other than Bob.               ║                   │        
        │ ╚═════════════════════════╤════╝                   │        
        ────┐                       │                        │        
        |   │ PeerDiscovery@Bob     │                        │        
        <───┘                       │                        │        
        │                           │                        │        
        │                           │sends message for Alice │        
        │                           │<───────────────────────│        
        │                           │                        │        
        │                           │ ╔══════════════════════╧═══════╗
        │                           │ ║Bob now has to open          ░║
        │                           │ ║a channel pointing to Alice,  ║
        │                           │ ║but coming from Charlie       ║
        │                           │ ╚══════════════════════╤═══════╝
        │     msgChannelCreate      │                        │        
        │<──────────────────────────│                        │        
        │                           │                        │        
        │Forwards Charlie's message │                        │        
        │<──────────────────────────│                        │        
     ┌──┴──┐                      ┌─┴─┐                  ┌───┴───┐    
     │Alice│                      │Bob│                  │Charlie│    
     └─────┘                      └───┘                  └───────┘   
```
