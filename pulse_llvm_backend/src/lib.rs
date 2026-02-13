//! LLVM Backend for the Pulse programming language
//!
//! This crate provides an LLVM-based compiler backend for Pulse,
//! allowing for ahead-of-time (AOT) compilation and just-in-time (JIT) execution.

pub mod backend;
pub mod jit;

pub use backend::LLVMBackend;
pub use jit::{JITCompiler, JITError, JITResult, JITStats, quick_compile};
