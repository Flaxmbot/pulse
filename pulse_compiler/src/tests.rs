#[cfg(test)]
mod tests {
    use crate::parser_v2::ParserV2;
    use crate::llvm_codegen::LLVMCodegen;
    use inkwell::context::Context;

    #[test]
    fn test_parse_simple() {
        let source = "let x = 10; fn add(a, b) { return a + b; } print add(x, 5);";
        let mut parser = ParserV2::new(source);
        let script = parser.parse().unwrap();
        assert_eq!(script.declarations.len(), 3);
    }

    #[test]
    fn test_llvm_codegen() {
        let source = "fn add(a, b) { return a + b; }";
        let mut parser = ParserV2::new(source);
        let script = parser.parse().unwrap();
        
        let context = Context::create();
        let mut codegen = LLVMCodegen::new(&context, "test_module");
        codegen.gen_script(&script).unwrap();
        
        codegen.print_to_stderr();
        assert!(codegen.verify());
    }
}
