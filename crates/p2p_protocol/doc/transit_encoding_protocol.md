# Lib3h Transit Encoding Protocol

```sequence
@startuml
Note right of Alice: Step 1: Open a connection\nsending config info\nand our kx pubkey
Alice->Bob: MsgStep1FromConnect
Note right of Bob: Bob can drop if we're\nworried about DDoS,\nor respond with\nHalt or Continue
Note right of Alice: Step 2: listening node's\nkx pubkey, sig pubkey,\nplus l2c session key
Bob->Alice: MsgStep2FromListen
Note right of Alice: Step 3: connect node's\nsig pubkey, c2l session key,\nplus signature
Alice->Bob: MsgStep3FromConnect
Note right of Bob: Bob verifies that Alice\nsigned his session key\nwith her sig private key
Note right of Alice: Step 4: Bob responds with\nhis signature
Bob->Alice: MsgStep4FromListen
Note right of Alice: Step 5: Accept Signature
Alice->Bob: MsgStep5FromConnect
Note right of Bob: Now that Alice has\naccepted Bob's signature,\nwe can continue exchanging\nencrypted messages
@enduml
```

http://www.plantuml.com/plantuml

```
     ┌─────┐               ┌───┐                             
     │Alice│               │Bob│                             
     └──┬──┘               └─┬─┘                             
        │ ╔══════════════════╧════════╗                      
        │ ║Step 1: Open a connection ░║                      
        │ ║sending config info        ║                      
        │ ║and our kx pubkey          ║                      
        │ ╚══════════════════╤════════╝                      
        │MsgStep1FromConnect │                               
        │───────────────────>│                               
        │                    │                               
        │                    │ ╔═══════════════════════╗     
        │                    │ ║Bob can drop if we're ░║     
        │                    │ ║worried about DDoS,    ║     
        │                    │ ║or respond with        ║     
        │                    │ ║Halt or Continue       ║     
        │                    │ ╚═══════════════════════╝     
        │ ╔══════════════════╧═══════╗                       
        │ ║Step 2: listening node's ░║                       
        │ ║kx pubkey, sig pubkey,    ║                       
        │ ║plus l2c session key      ║                       
        │ ╚══════════════════╤═══════╝                       
        │MsgStep2FromListen  │                               
        │<───────────────────│                               
        │                    │                               
        │ ╔══════════════════╧═══════════╗                   
        │ ║Step 3: connect node's       ░║                   
        │ ║sig pubkey, c2l session key,  ║                   
        │ ║plus signature                ║                   
        │ ╚══════════════════╤═══════════╝                   
        │MsgStep3FromConnect │                               
        │───────────────────>│                               
        │                    │                               
        │                    │ ╔══════════════════════════╗  
        │                    │ ║Bob verifies that Alice  ░║  
        │                    │ ║signed his session key    ║  
        │                    │ ║with her sig private key  ║  
        │                    │ ╚══════════════════════════╝  
        │ ╔══════════════════╧════════╗                      
        │ ║Step 4: Bob responds with ░║                      
        │ ║his signature              ║                      
        │ ╚══════════════════╤════════╝                      
        │MsgStep4FromListen  │                               
        │<───────────────────│                               
        │                    │                               
        │ ╔══════════════════╧═══════╗                       
        │ ║Step 5: Accept Signature ░║                       
        │ ╚══════════════════╤═══════╝                       
        │MsgStep5FromConnect │                               
        │───────────────────>│                               
        │                    │                               
        │                    │ ╔════════════════════════════╗
        │                    │ ║Now that Alice has         ░║
        │                    │ ║accepted Bob's signature,   ║
        │                    │ ║we can continue exchanging  ║
        │                    │ ║encrypted messages          ║
     ┌──┴──┐               ┌─┴─╚════════════════════════════╝
     │Alice│               │Bob│                             
     └─────┘               └───┘                             
```
