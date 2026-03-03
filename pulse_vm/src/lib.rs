pub mod debug;
pub mod heap;
pub mod shared_heap;
pub mod vm;
// mod send_check;

pub use debug::{DebugContext, StepMode};
pub use heap::Heap;
pub use shared_heap::{create_shared_heap, SharedHandle, SharedHeap};
pub use vm::{
    CallFrame, Capability, ResourceLimits, ResourceTracker, SecurityContext, VMStatus, VM,
};

#[cfg(test)]
mod tests {
    use super::*;
    use pulse_core::{ActorId, Chunk, Constant, Op, Value};

    #[tokio::test]
    async fn test_add() {
        let mut chunk = Chunk::new();
        let idx1 = chunk.add_constant(Constant::Int(10));
        let idx2 = chunk.add_constant(Constant::Int(32));

        // PUSH 10, PUSH 32, ADD, HALT
        // VM uses u16 for constant indices, so we write two bytes (little endian)
        chunk.write(Op::Const as u8, 1);
        chunk.write((idx1 & 0xFF) as u8, 1);
        chunk.write(((idx1 >> 8) & 0xFF) as u8, 1);

        chunk.write(Op::Const as u8, 1);
        chunk.write((idx2 & 0xFF) as u8, 1);
        chunk.write(((idx2 >> 8) & 0xFF) as u8, 1);

        chunk.write(Op::Add as u8, 2);

        chunk.write(Op::Halt as u8, 4);

        let pid = ActorId::new(1, 1);
        let mut vm = VM::new(chunk, pid, None);
        let status = vm.run(100).await;

        assert!(matches!(status, VMStatus::Halted));

        let result = vm.pop().unwrap();
        assert_eq!(result, Value::Int(42));
    }
}
