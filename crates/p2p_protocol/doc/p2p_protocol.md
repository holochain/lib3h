# Lib3h P2p Protocol

```sequence
@startuml
participant Alice
participant Bob
note right of Bob: Tell each other about\nrrdht arc lengths, &&\neventualy QoS data\naggregation exchange
Alice -> Bob: msgHandshake
Bob -> Alice: msgHandshake
note right of Alice: Alice determines constraints\nsuch as overlaping arcs,\ngathers all aspect hashes\nthat match those constraints
Alice -> Bob: msgGspArcRequest
note right of Bob: Bob requests data for\nall hashes he doesn't have
Bob -> Alice: msgGspAspectDataRequest
note right of Alice: Alice responds with\naspect data
Alice -> Bob: msgGspAspectDataResponse
note right of Bob: Bob also sends back\na list of hashes he has\nwithin the constraints that\nAlice didn't send
Bob -> Alice: msgGspArcResponse
note right of Alice: Alice may have received\nsome of these hashes in the mean\ntime, she requests those she\nstill needs
Alice -> Bob: msgGspAspectDataRequest
Bob -> Alice: msgGspAspectDataResponse
note right of Alice: If Alice authors new data\nshe can broadcast it to bob\nand others
Alice -> Bob: msgGspAspectBroadcast
@enduml
```

http://www.plantuml.com/plantuml

```
     ┌─────┐                    ┌───┐                              
     │Alice│                    │Bob│                              
     └──┬──┘                    └─┬─┘                              
        │                         │ ╔═══════════════════════╗      
        │                         │ ║Tell each other about ░║      
        │                         │ ║rrdht arc lengths, &&  ║      
        │                         │ ║eventualy QoS data     ║      
        │                         │ ║aggregation exchange   ║      
        │                         │ ╚═══════════════════════╝      
        │      msgHandshake       │                                
        │────────────────────────>│                                
        │                         │                                
        │      msgHandshake       │                                
        │<────────────────────────│                                
        │                         │                                
        │ ╔═══════════════════════╧══════╗                         
        │ ║Alice determines constraints ░║                         
        │ ║such as overlaping arcs,      ║                         
        │ ║gathers all aspect hashes     ║                         
        │ ║that match those constraints  ║                         
        │ ╚═══════════════════════╤══════╝                         
        │    msgGspArcRequest     │                                
        │────────────────────────>│                                
        │                         │                                
        │                         │ ╔════════════════════════════╗ 
        │                         │ ║Bob requests data for      ░║ 
        │                         │ ║all hashes he doesn't have  ║ 
        │                         │ ╚════════════════════════════╝ 
        │msgGspAspectDataRequest  │                                
        │<────────────────────────│                                
        │                         │                                
        │ ╔═════════════════════╗ │                                
        │ ║Alice responds with ░║ │                                
        │ ║aspect data          ║ │                                
        │ ╚═════════════════════╝ │                                
        │msgGspAspectDataResponse │                                
        │────────────────────────>│                                
        │                         │                                
        │                         │ ╔═════════════════════════════╗
        │                         │ ║Bob also sends back         ░║
        │                         │ ║a list of hashes he has      ║
        │                         │ ║within the constraints that  ║
        │                         │ ║Alice didn't send            ║
        │                         │ ╚═════════════════════════════╝
        │   msgGspArcResponse     │                                
        │<────────────────────────│                                
        │                         │                                
        │ ╔═══════════════════════╧══════════╗                     
        │ ║Alice may have received          ░║                     
        │ ║some of these hashes in the mean  ║                     
        │ ║time, she requests those she      ║                     
        │ ║still needs                       ║                     
        │ ╚═══════════════════════╤══════════╝                     
        │msgGspAspectDataRequest  │                                
        │────────────────────────>│                                
        │                         │                                
        │msgGspAspectDataResponse │                                
        │<────────────────────────│                                
        │                         │                                
        │ ╔═══════════════════════╧═════╗                          
        │ ║If Alice authors new data   ░║                          
        │ ║she can broadcast it to bob  ║                          
        │ ║and others                   ║                          
        │ ╚═══════════════════════╤═════╝                          
        │ msgGspAspectBroadcast   │                                
        │────────────────────────>│                                
     ┌──┴──┐                    ┌─┴─┐                              
     │Alice│                    │Bob│                              
     └─────┘                    └───┘                              
```
