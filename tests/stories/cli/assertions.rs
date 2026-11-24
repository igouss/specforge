use std::path::PathBuf;

use cucumber::then;
use serde_json::Value;
use specforge::cli::CliOutput;
use specforge::ledger_fs::FsLedger;

use crate::Forge;

fn last_output(world: &Forge) -> &CliOutput {
    world.cli.last_output.as_ref().expect("CLI command ran")
}

fn parsed_json(world: &Forge) -> Value {
    let stdout: &str = &last_output(world).stdout;
    serde_json::from_str::<Value>(stdout).expect("CLI stdout is valid JSON")
}

#[then(expr = "the CLI exits with code {int}")]
fn cli_exit_code(world: &mut Forge, expected: u8) {
    let output: &CliOutput = last_output(world);
    assert_eq!(
        output.exit_code, expected,
        "unexpected exit code; stdout:\n{}\nstderr:\n{}",
        output.stdout, output.stderr
    );
}

#[then(expr = "the CLI stdout contains {string}")]
fn cli_stdout_contains(world: &mut Forge, needle: String) {
    let stdout: &str = &last_output(world).stdout;
    assert!(stdout.contains(&needle), "expected stdout to contain \"{needle}\", got:\n{stdout}");
}

#[then(expr = "the CLI stderr contains {string}")]
fn cli_stderr_contains(world: &mut Forge, needle: String) {
    let stderr: &str = &last_output(world).stderr;
    assert!(stderr.contains(&needle), "expected stderr to contain \"{needle}\", got:\n{stderr}");
}

#[then(expr = "the CLI JSON field {string} is a 64-character hex digest")]
fn cli_json_digest(world: &mut Forge, field: String) {
    let json: Value = parsed_json(world);
    let value: &str = json.get(&field).and_then(Value::as_str).expect("string field exists");
    assert_eq!(value.len(), 64, "expected 64-char hex digest, got {value}");
    assert!(value.chars().all(|c: char| c.is_ascii_hexdigit()), "non-hex digest {value}");
}

#[then(expr = "the CLI JSON object {string} contains key {string}")]
fn cli_json_object_has_key(world: &mut Forge, field: String, key: String) {
    let json: Value = parsed_json(world);
    let object: &serde_json::Map<String, Value> = json.get(&field).and_then(Value::as_object).expect("object field exists");
    assert!(object.contains_key(&key), "expected key {key} in {object:?}");
}

#[then(expr = "the CLI JSON field {string} equals {string}")]
fn cli_json_string_equals(world: &mut Forge, field: String, expected: String) {
    let json: Value = parsed_json(world);
    let actual: &str = json.get(&field).and_then(Value::as_str).expect("string field exists");
    assert_eq!(actual, expected);
}

#[then(expr = "the CLI JSON field {string} equals boolean true")]
fn cli_json_bool_true(world: &mut Forge, field: String) {
    let json: Value = parsed_json(world);
    let actual: bool = json.get(&field).and_then(Value::as_bool).expect("bool field exists");
    assert!(actual, "expected {field} to be true");
}

#[then(expr = "the CLI JSON array {string} has length {int}")]
fn cli_json_array_len(world: &mut Forge, field: String, expected: usize) {
    let json: Value = parsed_json(world);
    let array: &Vec<Value> = json.get(&field).and_then(Value::as_array).expect("array field exists");
    assert_eq!(array.len(), expected, "array {field} was {array:?}");
}

#[then("the multi-file compile snapshot digest matches sorted source compilation")]
fn cli_multi_file_digest_matches(world: &mut Forge) {
    let json: Value = parsed_json(world);
    let actual: &str = json.get("snapshot_id").and_then(Value::as_str).expect("snapshot_id exists");
    let expected: &str = world.cli.expected_multi_snapshot.as_deref().expect("expected snapshot digest saved");
    assert_eq!(actual, expected);
}

#[then("the empty ledger directory has no blobs subdirectory created")]
fn cli_empty_ledger_no_blobs(world: &mut Forge) {
    let ledger_dir: &PathBuf = world.cli.ledger_dir.as_ref().expect("ledger dir exists");
    assert!(
        !ledger_dir.join("blobs").exists(),
        "read-only CLI command must not create blobs/ in {}",
        ledger_dir.display()
    );
}

#[then("the CLI ledger contains a persisted counterexample blob")]
fn cli_ledger_has_blob(world: &mut Forge) {
    let json: Value = parsed_json(world);
    let digest: &str = json
        .get("counterexample_blob_digest")
        .and_then(Value::as_str)
        .expect("counterexample_blob_digest present");
    let ledger_dir: &PathBuf = world.cli.ledger_dir.as_ref().expect("ledger dir exists");
    let blob_path: PathBuf = ledger_dir.join("blobs").join(digest);
    assert!(blob_path.is_file(), "expected blob file at {}", blob_path.display());
    let reopened: FsLedger = FsLedger::open_read_only(ledger_dir.clone()).expect("ledger reopens cleanly");
    assert_eq!(reopened.ledger().records().len(), 1);
}

#[then("the CLI stdout matches the canonical ledger export JSON")]
fn cli_stdout_matches_export(world: &mut Forge) {
    let ledger_dir: &PathBuf = world.cli.ledger_dir.as_ref().expect("ledger dir exists");
    let reopened: FsLedger = FsLedger::open_read_only(ledger_dir.clone()).expect("ledger opens");
    let expected: String = reopened.ledger().export_json().expect("canonical export");
    assert_eq!(last_output(world).stdout.trim_end_matches('\n'), expected);
}
