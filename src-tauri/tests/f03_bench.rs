use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// SPEC_DEVIATION: the binary path is `CARGO_BIN_EXE_f03-bench`.
// Reason: Cargo keeps the target name exactly, including the hyphen.

#[test]
fn missing_arguments_print_usage_and_create_nothing() {
    let scratch = Scratch::new("usage");
    let marker = scratch.path.join("marker.bin");
    fs::write(&marker, b"keep").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_f03-bench"))
        .output()
        .expect("binary");
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(String::from_utf8_lossy(&output.stderr).trim(), "usage");
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read(&marker).unwrap(), b"keep");
}

#[test]
fn count_zero_prints_the_error_code_and_does_not_create_the_root() {
    let scratch = Scratch::new("zero");
    let root = scratch.path.join("corpus");
    let root_arg = root.to_string_lossy().into_owned();
    let output = Command::new(env!("CARGO_BIN_EXE_f03-bench"))
        .args(["--root", &root_arg, "--count", "0"])
        .output()
        .expect("binary");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr).trim(),
        "count_out_of_range"
    );
    assert!(!root.exists());
}

#[test]
fn count_one_writes_pretty_json_and_removes_the_root() {
    let scratch = Scratch::new("one");
    let root = scratch.path.join("corpus");
    let output_path = scratch.path.join("report.json");
    let root_arg = root.to_string_lossy().into_owned();
    let output_arg = output_path.to_string_lossy().into_owned();
    let output = Command::new(env!("CARGO_BIN_EXE_f03-bench"))
        .args([
            "--root",
            &root_arg,
            "--count",
            "1",
            "--output",
            &output_arg,
        ])
        .output()
        .expect("binary");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let file = fs::read(&output_path).unwrap();
    assert_eq!(output.stdout, file);
    let text = String::from_utf8(file).unwrap();
    assert!(text.contains('\n'));
    assert!(text.contains("\"itemCount\": 1"));
    assert!(text.contains("\"bytesPerItem\": 4096"));
    assert!(text.contains("\"filesHashed\": 1"));
    assert!(text.contains("\"ok\": true"));
    assert!(!text.contains("item_count"));
    assert!(!root.exists());
}

struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "my-photos-f03-cli-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(&path).unwrap();
        Self { path }
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
