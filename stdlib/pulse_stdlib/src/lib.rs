#![allow(ambiguous_glob_reexports)]
//! Pulse Standard Library - Core Native Functions
//!
//! This is the reduced core standard library (80% reduction from original).
//! Only essential modules are included:
//! - io: File I/O operations
//! - json: JSON parsing/serialization
//! - http: Basic HTTP client
//! - time: Time utilities
//! - string_utils: String utilities
//! - collections: List/Map utilities
//! - fs: Filesystem operations
//! - utils: General utilities
//! - testing: Test framework

pub mod collections;
pub mod fs;
pub mod http;
pub mod io;
pub mod json;
pub mod string_utils;
pub mod testing;
pub mod time;
pub mod utils;

// Re-export core module functions
pub use collections::*;
pub use fs::*;
pub use http::*;
pub use io::*;
pub use json::*;
pub use string_utils::*;
pub use testing::*;
pub use time::*;
pub use utils::*;

/// Initialize the standard library
pub fn init() {
    // Any initialization needed for the standard library
}

/// Get the version of the standard library
pub fn version() -> &'static str {
    "2.0.0-core"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert_eq!(version(), "2.0.0-core");
    }
}
pub mod crypto;
pub mod tcp;
pub mod websocket;
