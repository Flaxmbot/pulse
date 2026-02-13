pub mod vm;
pub mod heap;
pub mod debug;
// mod send_check;

pub use vm::{VM, VMStatus, CallFrame};
pub use heap::Heap;
pub use debug::{DebugContext, StepMode};

#[cfg(test)]
mod tests {
    use super::*;
    use pulse_core::{Chunk, Op, Value, Constant, ActorId};

    #[test]
    fn test_add() {
        let mut chunk = Chunk::new();
        let idx1 = chunk.add_constant(Constant::Int(10));
        let idx2 = chunk.add_constant(Constant::Int(32));

        // PUSH 10, PUSH 32, ADD, HALT
        chunk.write(Op::Const as u8, 1);
        chunk.write(idx1 as u8, 1);

        chunk.write(Op::Const as u8, 1);
        chunk.write(idx2 as u8, 1);

        chunk.write(Op::Add as u8, 2);

        chunk.write(Op::Halt as u8, 4);

        let pid = ActorId::new(1, 1);
        let mut vm = VM::new(chunk, pid);
        let status = vm.run(100);

        assert!(matches!(status, VMStatus::Halted));

        let result = vm.pop().unwrap();
        assert_eq!(result, Value::Int(42));
    }
}
