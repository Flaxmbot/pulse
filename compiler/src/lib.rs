#[path = "ast/ast.rs"]
pub mod ast;
#[path = "ast/ast_lowering.rs"]
pub mod ast_lowering;
#[path = "codegen/compiler.rs"]
pub mod compiler;
#[path = "parser/lexer.rs"]
pub mod lexer;
#[path = "parser/parser_v2.rs"]
pub mod parser_v2;
#[path = "semantic/type_checker.rs"]
pub mod type_checker;
#[path = "semantic/types.rs"]
pub mod types;

pub use compiler::compile;
pub use compiler::Compiler;
pub use lexer::{Lexer, Token};
pub use parser_v2::ParserV2;
pub use type_checker::{TypeChecker, TypeError};
pub use types::Type;