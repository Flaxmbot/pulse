use serde::{Serialize, Deserialize};
use crate::value::Constant;
// use crate::value::Value; // No longer used in Chunk constants

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum Op {
    // Basics
    Halt = 0x00,
    Const = 0x01,
    Pop = 0x02,
    Negate = 0x03,
    Not = 0x04,
    Unit = 0x05,
    Dup = 0x06,
    IsList = 0x07,
    IsMap = 0x08,
    Slice = 0x09,
    Len = 0x0A,
    MapContainsKey = 0x0B,
    Slide = 0x0C,
    ToString = 0x0D,

    // Arithmetic
    Add = 0x10,
    Sub = 0x11,
    Mul = 0x12,
    Div = 0x13,
    Mod = 0x14,

    // Comparison
    Eq = 0x20,
    Neq = 0x21,
    Gt = 0x22,
    Lt = 0x23,

    // Logic
    And = 0x24,
    Or = 0x25,

    // Control Flow
    Jump = 0x30,
    JumpIfFalse = 0x31,
    Call = 0x32,
    Loop = 0x33,
    Return = 0x34,
    Closure = 0x35,
    GetUpvalue = 0x36,
    SetUpvalue = 0x37,
    CloseUpvalue = 0x38,

    // Variables
    GetLocal = 0x40,
    SetLocal = 0x41,
    GetGlobal = 0x42,
    SetGlobal = 0x43,
    DefineGlobal = 0x44,

    // Data Structures
    BuildList = 0x45, // u8 count. Pops count items, pushes List handle.
    BuildMap = 0x46,  // u8 count. Pops count * 2 (key, val) items, pushes Map handle.
    GetIndex = 0x47,  // Pops index, target. Pushes value.
    SetIndex = 0x48,  // Pops value, index, target. Sets value.

    // Distribution
    Spawn = 0x50,     // u8 constant index for function.
    Send = 0x51,
    Receive = 0x52,
    SelfId = 0x53,
    Import = 0x54,
    SpawnLink = 0x55,
    Link = 0x56,
    Monitor = 0x57,
    Register = 0x58,
    Unregister = 0x59,
    WhereIs = 0x5A,
    
    // Shared Memory
    CreateSharedMemory = 0x5B,  // Create shared memory region
    ReadSharedMemory = 0x5C,    // Read from shared memory
    WriteSharedMemory = 0x5D,   // Write to shared memory
    LockSharedMemory = 0x5E,    // Acquire lock on shared memory
    UnlockSharedMemory = 0x5F,  // Release lock on shared memory
    
    // IO
    Print = 0x60,
    PrintMulti = 0x61,

    // Error Handling
    Try = 0x70,      // u16 handler offset. Push exception handler frame.
    Throw = 0x71,    // Pop value, unwind to handler.
    EndTry = 0x72,   // Pop exception handler frame.

    // Object Oriented
    BuildClass = 0x73,  // u8 name_idx, u8 has_super, [u8 super_idx if has_super], u8 method_count, [u8 method_name_idx for each method]
    GetSuper = 0x74,    // u16 method_name_idx. Pops super, this. Pushes BoundMethod.
    Method = 0x75,      // u16 name_idx. Pops closure, peeks class, adds method.
    
    // Atomic Operations
    AtomicCreate = 0x76,  // Create atomic int, pops initial value
    AtomicLoad = 0x77,    // Load from atomic, pushes value
    AtomicStore = 0x78,   // Store to atomic, pops value, pushes old value
    AtomicAdd = 0x79,     // Atomic add, pops value, pushes old value
    AtomicSub = 0x7A,     // Atomic subtract, pops value, pushes old value
    AtomicCompareAndSwap = 0x7B,  // CAS, pops expected, new, pushes success (bool)
    
    // Memory Fences
    MemoryFenceAcquire = 0x7C,  // Acquire fence - all reads after this see writes before matching release
    MemoryFenceRelease = 0x7D,  // Release fence - all writes before this are visible to threads doing acquire
    MemoryFenceSeqCst = 0x7E,  // Full memory barrier (Sequentially consistent)
}

impl From<u8> for Op {
    fn from(byte: u8) -> Self {
        match byte {
            0x00 => Op::Halt,
            0x01 => Op::Const,
            0x02 => Op::Pop,
            0x03 => Op::Negate,
            0x04 => Op::Not,
            0x05 => Op::Unit,
            0x06 => Op::Dup,
            0x07 => Op::IsList,
            0x08 => Op::IsMap,
            0x09 => Op::Slice,
            0x0A => Op::Len,
            0x0B => Op::MapContainsKey,
            0x0C => Op::Slide,
            0x0D => Op::ToString,
            0x10 => Op::Add,
            0x11 => Op::Sub,
            0x12 => Op::Mul,
            0x13 => Op::Div,
            0x14 => Op::Mod,
            0x20 => Op::Eq,
            0x21 => Op::Neq,
            0x22 => Op::Gt,
            0x23 => Op::Lt,
            0x24 => Op::And,
            0x25 => Op::Or,
            0x30 => Op::Jump,
            0x31 => Op::JumpIfFalse,
            0x32 => Op::Call,
            0x33 => Op::Loop,
            0x34 => Op::Return,
            0x35 => Op::Closure,
            0x36 => Op::GetUpvalue,
            0x37 => Op::SetUpvalue,
            0x38 => Op::CloseUpvalue,
            0x40 => Op::GetLocal,
            0x41 => Op::SetLocal,
            0x42 => Op::GetGlobal,
            0x43 => Op::SetGlobal,
            0x44 => Op::DefineGlobal,
            0x45 => Op::BuildList,
            0x46 => Op::BuildMap,
            0x47 => Op::GetIndex,
            0x48 => Op::SetIndex,
            0x50 => Op::Spawn,
            0x51 => Op::Send,
            0x52 => Op::Receive,
            0x53 => Op::SelfId,
            0x54 => Op::Import,
            0x55 => Op::SpawnLink,
            0x56 => Op::Link,
            0x57 => Op::Monitor,
            0x58 => Op::Register,
            0x59 => Op::Unregister,
            0x5A => Op::WhereIs,
            0x5B => Op::CreateSharedMemory,
            0x5C => Op::ReadSharedMemory,
            0x5D => Op::WriteSharedMemory,
            0x5E => Op::LockSharedMemory,
            0x5F => Op::UnlockSharedMemory,
            0x60 => Op::Print,
            0x61 => Op::PrintMulti,
            0x70 => Op::Try,
            0x71 => Op::Throw,
            0x72 => Op::EndTry,
            0x73 => Op::BuildClass,
            0x74 => Op::GetSuper,
            0x75 => Op::Method,
            0x76 => Op::AtomicCreate,
            0x77 => Op::AtomicLoad,
            0x78 => Op::AtomicStore,
            0x79 => Op::AtomicAdd,
            0x7A => Op::AtomicSub,
            0x7B => Op::AtomicCompareAndSwap,
            0x7C => Op::MemoryFenceAcquire,
            0x7D => Op::MemoryFenceRelease,
            0x7E => Op::MemoryFenceSeqCst,
            _ => Op::Halt, // Default to halt for invalid opcodes
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Constant>,
    pub lines: Vec<usize>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn write(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: Constant) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }
    
    pub fn get_constant(&self, index: usize) -> Option<&Constant> {
        self.constants.get(index)
    }
}
