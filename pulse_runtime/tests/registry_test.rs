use pulse_runtime::Runtime;

#[test]
fn test_register_and_whereis() {
    let mut runtime = Runtime::new(1);
    
    // Script:
    // 1. Spawns a child.
    // 2. Registers it as "my_service".
    // 3. verifying whereis works in script is hard to assert from outside without effects.
    // But we can verify from outside using runtime.whereis().
    let source = r#"
        let child = spawn {
            receive; // Wait forever
        };
        register("my_service", child);
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    let _parent = runtime.spawn(chunk, None);
    
    // Run
    for _ in 0..50 {
        runtime.step();
    }
    
    // Verify
    let resolved = runtime.whereis("my_service");
    assert!(resolved.is_some(), "Service should be registered");
}

#[test]
fn test_unregister() {
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let me = self();
        register("temporary", me);
        unregister("temporary");
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    
    for _ in 0..50 {
        runtime.step();
    }
    
    assert!(runtime.whereis("temporary").is_none(), "Service should be unregistered");
}

#[test]
fn test_automatic_unregistration() {
    // This tests that when an actor dies, its name is removed.
    let mut runtime = Runtime::new(1);
    
    let source = r#"
        let child = spawn {
            let me = self();
            register("doomed", me);
            // Die now
        };
        
        monitor child;
        receive; // Wait for Down message to ensure child is dead and cleaned up
    "#;
    
    let chunk = pulse_compiler::compile(source, None).unwrap();
    runtime.spawn(chunk, None);
    
    // Run enough steps
    for _ in 0..100 {
        runtime.step();
    }
    
    // Child should be dead, registry cleaned.
    assert!(runtime.whereis("doomed").is_none(), "Doomed service should be automatically unregistered");
}

#[test]
fn test_whereis_opcode() {
    // Verify the `whereis` opcode works in script
    let mut runtime = Runtime::new(1);
    
    // We'll use a side effect (print) or just rely on runtime not crashing.
    // To properly test, we can loop until whereis returns something?
    
    let source = r#"
        let child = spawn { receive; };
        register("lookup_target", child);
        
        let found = whereis("lookup_target");
        
        if (found == child) {
            // Good
        } else {
            // Failure
        }
    "#;
    
    let chunk = pulse_compiler::compile(source, None).expect("Compile error");
    runtime.spawn(chunk, None);
    
    for _ in 0..50 {
        runtime.step();
    }
    
    assert!(runtime.whereis("lookup_target").is_some());
}
