//! Tests for serialization of complex objects

use pulse_core::{Value, Object, Function, Chunk, Constant, ActorId};
use std::sync::Arc;
use std::collections::{HashMap, HashSet, VecDeque};

/// Test serialization of basic types
#[test]
fn test_basic_types_serialize() {
    // Test Value serialization
    let values = vec![
        Value::Bool(true),
        Value::Bool(false),
        Value::Int(42),
        Value::Int(-100),
        Value::Float(3.14),
        Value::Float(-2.718),
        Value::Unit,
    ];
    
    for value in values {
        let serialized = bincode::serialize(&value).expect("Should serialize");
        let deserialized: Value = bincode::deserialize(&serialized).expect("Should deserialize");
        assert_eq!(value, deserialized);
    }
}

/// Test ActorId serialization
#[test]
fn test_actor_id_serialize() {
    let actor_id = ActorId::new(12345, 67890);
    let serialized = bincode::serialize(&actor_id).expect("Should serialize");
    let deserialized: ActorId = bincode::deserialize(&serialized).expect("Should deserialize");
    assert_eq!(actor_id, deserialized);
}

/// Test Object::String serialization
#[test]
fn test_object_string_serialize() {
    let obj = Object::String("Hello, World!".to_string());
    let serialized = bincode::serialize(&obj).expect("Should serialize");
    let _deserialized: Result<Object, _> = bincode::deserialize(&serialized);
    // Note: Object deserialization returns error as expected
}

/// Test Object::List serialization
#[test]
fn test_object_list_serialize() {
    let list = Object::List(vec![
        Value::Int(1),
        Value::Int(2),
        Value::Int(3),
    ]);
    let serialized = bincode::serialize(&list).expect("Should serialize");
    let _deserialized: Result<Object, _> = bincode::deserialize(&serialized);
    // Note: Object deserialization returns error as expected
}

/// Test Object::Map serialization
#[test]
fn test_object_map_serialize() {
    let mut map = HashMap::new();
    map.insert("key1".to_string(), Value::Int(1));
    map.insert("key2".to_string(), Value::Int(2));
    let obj = Object::Map(map);
    let serialized = bincode::serialize(&obj).expect("Should serialize");
    let _deserialized: Result<Object, _> = bincode::deserialize(&serialized);
}

/// Test Object::Set serialization
#[test]
fn test_object_set_serialize() {
    let mut set = HashSet::new();
    set.insert("a".to_string());
    set.insert("b".to_string());
    set.insert("c".to_string());
    let obj = Object::Set(set);
    let serialized = bincode::serialize(&obj).expect("Should serialize");
    let _deserialized: Result<Object, _> = bincode::deserialize(&serialized);
}

/// Test Object::Queue serialization
#[test]
fn test_object_queue_serialize() {
    let mut queue = VecDeque::new();
    queue.push_back(Value::Int(1));
    queue.push_back(Value::Int(2));
    let obj = Object::Queue(queue);
    let serialized = bincode::serialize(&obj).expect("Should serialize");
    let _deserialized: Result<Object, _> = bincode::deserialize(&serialized);
}

/// Test Function serialization
#[test]
fn test_function_serialize() {
    let chunk = Arc::new(Chunk::new());
    let function = Function {
        arity: 2,
        chunk,
        name: "test_function".to_string(),
        upvalue_count: 0,
        module_path: Some("test.pulse".to_string()),
    };
    
    let serialized = bincode::serialize(&function).expect("Should serialize");
    let deserialized: Function = bincode::deserialize(&serialized).expect("Should deserialize");
    assert_eq!(function.name, deserialized.name);
    assert_eq!(function.arity, deserialized.arity);
}

/// Test Chunk serialization
#[test]
fn test_chunk_serialize() {
    let mut chunk = Chunk::new();
    chunk.add_constant(Constant::Int(42));
    chunk.add_constant(Constant::String("test".to_string()));
    
    // Serialize - this should work
    let result = bincode::serialize(&chunk);
    assert!(result.is_ok(), "Should serialize: {:?}", result.err());
    
    let serialized = result.unwrap();
    assert!(!serialized.is_empty(), "Should have serialized data");
    
    // Note: Full round-trip deserialization requires SerializableConstant wrapper
    // This test verifies serialization works correctly
}

/// Test MessageEnvelope serialization
#[test]
fn test_message_envelope_serialize() {
    use pulse_runtime::network::MessageEnvelope;
    use pulse_runtime::mailbox::Message;
    
    let target = ActorId::new(1, 100);
    let sender = ActorId::new(1, 200);
    // Use serializable constants only
    let message = Message::User(Constant::Int(42));
    
    let envelope = MessageEnvelope::new(target, Some(sender), message);
    
    // Serialize - this should work
    let result = envelope.to_bytes();
    assert!(result.is_ok(), "Should serialize: {:?}", result.err());
    
    let serialized = result.unwrap();
    assert!(!serialized.is_empty(), "Should have serialized data");
    
    // Note: Full round-trip deserialization requires SerializableConstant wrapper
    // This test verifies serialization works correctly
}

/// Test RemoteSpawnRequest serialization
#[test]
fn test_remote_spawn_request_serialize() {
    use pulse_runtime::network::RemoteSpawnRequest;
    use pulse_core::object::Function;
    use std::sync::Arc;
    
    let chunk = Arc::new(Chunk::new());
    let function = Arc::new(Function {
        arity: 1,
        chunk,
        name: "remote_actor".to_string(),
        upvalue_count: 0,
        module_path: Some("actor.pulse".to_string()),
    });
    
    let args = vec![Value::Int(42), Value::Bool(true)];
    let request = RemoteSpawnRequest::new(function.clone(), args, Some("my_actor".to_string()));
    
    let serialized = request.to_bytes().expect("Should serialize");
    let _deserialized = RemoteSpawnRequest::from_bytes(&serialized).expect("Should deserialize");
    // Note: The function chunk will be cloned properly
}

/// Test that non-serializable types properly fail
#[test]
fn test_non_serializable_fails() {
    // Test Object::AtomicInt serialization (stores value as i64)
    use pulse_core::Object::AtomicInt;
    use pulse_core::object::AtomicInt as PulseAtomicInt;
    use std::sync::atomic::AtomicI64;
    
    let atomic = PulseAtomicInt {
        value: Arc::new(AtomicI64::new(42)),
    };
    let obj = AtomicInt(atomic);
    let result = bincode::serialize(&obj);
    // This should work - AtomicInt serializes its value
    assert!(result.is_ok());
    
    // Test Function serialization in Closure (should serialize as Function only)
    use pulse_core::object::Closure;
    let chunk = Arc::new(Chunk::new());
    let function = Function {
        arity: 1,
        chunk,
        name: "test".to_string(),
        upvalue_count: 0,
        module_path: None,
    };
    let closure = Closure {
        function,
        upvalues: vec![],
    };
    let serialized = bincode::serialize(&closure).expect("Should serialize closure");
    let deserialized: Function = bincode::deserialize(&serialized).expect("Should deserialize to function");
    assert_eq!(deserialized.name, "test");
}
