use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn parity_cases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("simple_let", "let x = 1; print(x);"),
        (
            "function_call",
            "fn add(a, b) { return a + b; } print(add(2, 3));",
        ),
        ("if_else", "if (true) { print(1); } else { print(2); }"),
        ("syntax_missing_rhs", "let x = ;"),
        ("syntax_missing_paren", "fn bad(a, b { return a; }"),
    ]
}

fn run_check(
    exe: &str,
    repo_root: &PathBuf,
    file: &PathBuf,
    track: &str,
    selfhost_entry: Option<&PathBuf>,
) -> (bool, String, String) {
    let mut cmd = Command::new(exe);
    cmd.arg("check")
        .arg(file)
        .arg("--diagnostics-json")
        .env("PULSE_COMPILER_TRACK", track)
        .current_dir(repo_root);
    if let Some(entry) = selfhost_entry {
        cmd.env("PULSE_SELFHOST_ENTRY", entry);
    }

    let output = cmd.output().expect("failed to execute pulse_cli check");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn selfhost_track_matches_rust_track_on_diagnostics_corpus() {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo_root = crate_dir
        .parent()
        .expect("pulse_cli crate should be under repository root")
        .to_path_buf();

    let corpus_dir = repo_root.join("target").join("parity_corpus");
    fs::create_dir_all(&corpus_dir).expect("failed to create parity corpus directory");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let case_file = corpus_dir.join(format!("parity_cases_{}.pulse", nonce));
    let bootstrap_stub = corpus_dir.join(format!("selfhost_bootstrap_stub_{}.pulse", nonce));

    let mut source = String::new();
    for (_, snippet) in parity_cases() {
        source.push_str(snippet);
        source.push('\n');
    }
    fs::write(&case_file, source).expect("failed to write parity corpus file");
    fs::write(&bootstrap_stub, "let selfhost_bootstrap_stub = 1;\n")
        .expect("failed to write bootstrap stub file");

    let exe = env!("CARGO_BIN_EXE_pulse_cli");
    let (rust_ok, rust_stdout, rust_stderr) = run_check(exe, &repo_root, &case_file, "rust", None);
    let (self_ok, self_stdout, self_stderr) = run_check(
        exe,
        &repo_root,
        &case_file,
        "selfhost",
        Some(&bootstrap_stub),
    );

    assert_eq!(
        self_ok, rust_ok,
        "exit status mismatch between tracks\nrust stderr:\n{}\nselfhost stderr:\n{}",
        rust_stderr, self_stderr
    );
    assert_eq!(
        self_stdout, rust_stdout,
        "diagnostics JSON mismatch between tracks\nrust stderr:\n{}\nselfhost stderr:\n{}",
        rust_stderr, self_stderr
    );
}
