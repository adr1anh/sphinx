use std::{
    fs::File,
    io::Write,
    process::{Command, Stdio},
};

use super::Constraint;

pub fn execute(constraints: &[Constraint]) {
    let serialized = serde_json::to_string(&constraints).unwrap();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let dir = format!("{}/../groth16", manifest_dir);
    let path = format!("{}/constraints.json", dir);
    let mut file = File::create(path).unwrap();
    file.write_all(serialized.as_bytes()).unwrap();

    let result = Command::new("go")
        .args([
            "test",
            "-v",
            "-timeout",
            "100000s",
            "-run",
            "^TestMain$",
            "github.com/succinctlabs/sp1-recursion-groth16",
        ])
        .current_dir(dir)
        .stderr(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stdin(Stdio::inherit())
        .output()
        .unwrap();

    assert!(result.status.success(), "failed to run test circuit");
}
