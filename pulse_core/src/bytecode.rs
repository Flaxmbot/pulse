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

    // Arithmetic
    Add = 0x10,
    Sub = 0x11,
    Mul = 0x12,
    Div = 0x13,

    // Comparison
    Eq = 0x20,
    Neq = 0x21,
    Gt = 0x22,
    Lt = 0x23,

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
    
    // IO
    Print = 0x60,
}

impl From<u8> for Op {
    fn from(byte: u8) -> Self {
        unsafe { std::mem::transmute(byte) }
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
}
