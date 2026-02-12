use std::time::Instant;
use std::fs;

fn main() {
    println!("Pulse Language Performance Benchmark Runner");
    println!("=========================================");

    // Benchmark the comprehensive test
    println!("\nRunning comprehensive keyword test...");
    let start = Instant::now();
    run_pulse_test("comprehensive_keyword_test.pulse");
    let duration = start.elapsed();
    println!("Comprehensive test completed in: {:?}", duration);

    // Benchmark the error handling test
    println!("\nRunning error handling test...");
    let start = Instant::now();
    run_pulse_test("error_handling_test.pulse");
    let duration = start.elapsed();
    println!("Error handling test completed in: {:?}", duration);

    // Benchmark the performance test
    println!("\nRunning performance benchmark...");
    let start = Instant::now();
    run_pulse_test("benchmark_test.pulse");
    let duration = start.elapsed();
    println!("Performance benchmark completed in: {:?}", duration);

    println!("\nAll benchmarks completed!");
}

fn run_pulse_test(filename: &str) {
    // Read the test file
    let source = fs::read_to_string(filename)
        .expect("Could not read test file");

    // Compile and run the test
    match pulse_compiler::compile(&source, Some(filename.to_string())) {
        Ok(chunk) => {
            let mut vm = pulse_vm::VM::new(chunk, pulse_core::ActorId::new(0, 1));
            let status = vm.run(1000000); // Run for up to 1M steps
            
            match status {
                pulse_vm::VMStatus::Halted | pulse_vm::VMStatus::Running => {
                    println!("  Test '{}' executed successfully", filename);
                }
                pulse_vm::VMStatus::Error(e) => {
                    println!("  Test '{}' failed with error: {:?}", filename, e);
                }
                _ => {
                    println!("  Test '{}' terminated with status: {:?}", filename, status);
                }
            }
        }
        Err(e) => {
            println!("  Failed to compile '{}': {:?}", filename, e);
        }
    }
}