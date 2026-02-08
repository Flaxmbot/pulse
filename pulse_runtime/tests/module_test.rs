use pulse_core::Value;
use pulse_runtime::{Runtime, Actor};
use pulse_compiler::compile;
use std::fs;
use std::sync::MutexGuard;

#[test]
fn test_module_import() {
    let math_pulse = "fn add(a, b) { return a + b; }";
    let main_pulse = "import \"math.pulse\"; let result = add(10, 20);";

    fs::write("math.pulse", math_pulse).expect("failed to write math.pulse");
    fs::write("main.pulse", main_pulse).expect("failed to write main.pulse");

    let mut runtime = Runtime::new(1);
    let chunk = compile(main_pulse, Some("main.pulse".into())).unwrap();
    let pid = runtime.spawn(chunk, Some("main.pulse".into()));

    for _ in 0..1000 {
        if !runtime.step() { break; }
    }

    if let Some(actor_ref) = runtime.get_actor_vm(pid) {
        let actor: MutexGuard<Actor> = actor_ref.lock().unwrap();
        let result = actor.vm.globals.get("result").expect("Global 'result' not found");
        assert_eq!(*result, Value::Int(30));
    } else {
        panic!("Actor not found");
    }

    let _ = fs::remove_file("math.pulse");
    let _ = fs::remove_file("main.pulse");
}

#[test]
fn test_circular_import() {
    let a_pulse = "import \"b_circ.pulse\"; let a = 1;";
    let b_pulse = "import \"a_circ.pulse\"; let b = 2;";
    
    fs::write("a_circ.pulse", a_pulse).expect("failed to write a_circ.pulse");
    fs::write("b_circ.pulse", b_pulse).expect("failed to write b_circ.pulse");

    let main_pulse = "import \"a_circ.pulse\"; import \"b_circ.pulse\"; let result = a + b;";     
    fs::write("circular_main.pulse", main_pulse).expect("failed to write circular_main.pulse");   

    let mut runtime = Runtime::new(1);
    let chunk = compile(main_pulse, Some("circular_main.pulse".into())).unwrap();
    let pid = runtime.spawn(chunk, Some("circular_main.pulse".into()));

    for _ in 0..1000 {
        if !runtime.step() { break; }
    }

    if let Some(actor_ref) = runtime.get_actor_vm(pid) {
        let actor: MutexGuard<Actor> = actor_ref.lock().unwrap();
        let result = actor.vm.globals.get("result").expect("Global 'result' not found");
        assert_eq!(*result, Value::Int(3));
    } else {
        panic!("Actor not found");
    }

    let _ = fs::remove_file("a_circ.pulse");
    let _ = fs::remove_file("b_circ.pulse");
    let _ = fs::remove_file("circular_main.pulse");
}
