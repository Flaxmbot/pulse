use pulse_core::{Chunk, Constant, Op, ActorId, Value};
use pulse_runtime::runtime::Runtime;
use pulse_runtime::mailbox::Message;
use pulse_runtime::actor::ActorStatus;
use std::time::Duration;
use std::thread;

#[test]
fn test_distributed_send() {
    // Node 1 (Receiver)
    let mut node1 = Runtime::new(1);
    node1.start_listener("127.0.0.1:9001").unwrap();

    // Node 2 (Sender)
    let mut node2 = Runtime::new(2);
    // Give node 1 a moment to start
    thread::sleep(Duration::from_millis(100));
    node2.connect(1, "127.0.0.1:9001").expect("Node 2 failed to connect to Node 1");

    // Spawn actor on Node 1 that waits for a message and then HALTs
    let mut chunk = Chunk::default();
    chunk.write(Op::Receive as u8, 1);
    chunk.write(Op::Pop as u8, 1); // Pop message
    chunk.write(Op::Halt as u8, 1);
    
    let pid1 = node1.spawn(chunk, None);
    assert_eq!(pid1.node_id, 1);

    // Send message from Node 2 to Node 1's actor
    node2.send(pid1, Message::User(Constant::Int(42))).expect("Failed to send message from Node 2");

    // Step Node 1 until actor receives and halts
    // Need a loop since networking is async
    let mut received = false;
    for _ in 0..100 {
        node1.step();
        
        let actor_ref = node1.get_actor_vm(pid1).unwrap();
        let actor = actor_ref.lock().unwrap();
        if actor.status == ActorStatus::Terminated {
            received = true;
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert!(received, "Actor on Node 1 never received message or never halted");
}
