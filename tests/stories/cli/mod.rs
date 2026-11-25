mod assertions;
mod extended;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use cucumber::{given, when};
use specforge::cli::{run, CliOutput};
use specforge::graph::{self, GraphSnapshot};
use specforge::trace::Source;

use crate::{fixture_text, Forge};

static CLI_WORKSPACES: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Default)]
pub struct CliState {
    pub workspace: Option<PathBuf>,
    pub valid_dir: Option<PathBuf>,
    pub defective_dir: Option<PathBuf>,
    pub safe_dir: Option<PathBuf>,
    pub violating_dir: Option<PathBuf>,
    pub unrelated_dir: Option<PathBuf>,
    pub property_edit_dir: Option<PathBuf>,
    pub claimless_dir: Option<PathBuf>,
    pub ledger_dir: Option<PathBuf>,
    pub expected_multi_snapshot: Option<String>,
    pub last_output: Option<CliOutput>,
}

pub(super) fn fresh_workspace() -> PathBuf {
    let index: usize = CLI_WORKSPACES.fetch_add(1, Ordering::SeqCst);
    let path: PathBuf = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli-stories").join(format!("ws-{index}"));
    if path.exists() {
        fs::remove_dir_all(&path).expect("previous workspace removed");
    }
    fs::create_dir_all(&path).expect("workspace created");
    path
}

pub(super) fn safe_variant(fixture: &str) -> String {
    let original: String = fixture_text(fixture);
    let removed: &str = "  - name: redeliver\n    from: acknowledged\n    to: redelivered\n    event: redeliver\n";
    assert!(original.contains(removed), "fixture {fixture} missing redeliver transition");
    original.replace(removed, "")
}

pub(super) fn write_spec_dir(root: &Path, subdir: &str, filename: &str, content: &str) -> PathBuf {
    let dir: PathBuf = root.join(subdir);
    fs::create_dir_all(&dir).expect("spec dir created");
    fs::write(dir.join(filename), content).expect("spec file written");
    dir
}

pub(super) fn path_arg(path: &Option<PathBuf>, label: &str) -> String {
    path.as_ref().expect(label).to_str().expect("utf-8 path").to_string()
}

fn active_verify_dir(world: &Forge) -> String {
    if let Some(dir) = world.cli.safe_dir.as_ref() {
        return dir.to_str().expect("utf-8 path").to_string();
    }
    path_arg(&world.cli.violating_dir, "violating spec dir exists")
}

#[given("a CLI workspace with a valid specification directory containing non-markdown and nested entries")]
fn workspace_valid_and_defective(world: &mut Forge) {
    let root: PathBuf = fresh_workspace();
    let valid: PathBuf = write_spec_dir(&root, "valid", "01-message.md", &fixture_text("sdv-message.md"));
    fs::write(valid.join("ignored.txt"), "not markdown").expect("non-markdown file written");
    let nested_md_dir: PathBuf = valid.join("nested.md");
    fs::create_dir_all(&nested_md_dir).expect("nested directory created");
    fs::write(nested_md_dir.join("child.md"), fixture_text("sdv-defective.md")).expect("nested file written");
    let defective: PathBuf = write_spec_dir(&root, "defective", "01-bad.md", &fixture_text("sdv-defective.md"));
    world.cli = CliState {
        workspace: Some(root),
        valid_dir: Some(valid),
        defective_dir: Some(defective),
        ..CliState::default()
    };
}

#[given("a CLI workspace with a safe specification directory and an empty ledger directory")]
fn workspace_safe_and_ledger(world: &mut Forge) {
    let root: PathBuf = fresh_workspace();
    let safe: PathBuf = write_spec_dir(&root, "safe", "01-message.md", &safe_variant("sdv-message.md"));
    let ledger: PathBuf = root.join("ledger");
    fs::create_dir_all(&ledger).expect("ledger dir created");
    world.cli = CliState {
        workspace: Some(root),
        safe_dir: Some(safe),
        ledger_dir: Some(ledger),
        ..CliState::default()
    };
}

#[given("a CLI workspace with a violating specification directory and an empty ledger directory")]
fn workspace_violating_and_ledger(world: &mut Forge) {
    let root: PathBuf = fresh_workspace();
    let violating: PathBuf = write_spec_dir(&root, "violating", "01-message.md", &fixture_text("sdv-message.md"));
    let ledger: PathBuf = root.join("ledger");
    fs::create_dir_all(&ledger).expect("ledger dir created");
    world.cli = CliState {
        workspace: Some(root),
        violating_dir: Some(violating),
        ledger_dir: Some(ledger),
        ..CliState::default()
    };
}

#[given("a CLI workspace with baseline, unrelated-edit, and property-edit safe specification directories")]
fn workspace_impact_dirs(world: &mut Forge) {
    let root: PathBuf = fresh_workspace();
    let safe: PathBuf = write_spec_dir(&root, "baseline", "01-message.md", &safe_variant("sdv-message.md"));
    let unrelated: PathBuf = write_spec_dir(&root, "unrelated", "01-message.md", &safe_variant("sdv-message-unrelated-edit.md"));
    let property_edit: PathBuf = write_spec_dir(&root, "property-edit", "01-message.md", &safe_variant("sdv-message-property-edit.md"));
    let ledger: PathBuf = root.join("ledger");
    fs::create_dir_all(&ledger).expect("ledger dir created");
    world.cli = CliState {
        workspace: Some(root),
        safe_dir: Some(safe),
        unrelated_dir: Some(unrelated),
        property_edit_dir: Some(property_edit),
        ledger_dir: Some(ledger),
        ..CliState::default()
    };
}

#[given("a CLI workspace with a claimless specification directory and an empty ledger directory")]
fn workspace_claimless(world: &mut Forge) {
    let root: PathBuf = fresh_workspace();
    let claimless: PathBuf = write_spec_dir(&root, "claimless", "01-req.md", &fixture_text("one-requirement.md"));
    let ledger: PathBuf = root.join("ledger");
    fs::create_dir_all(&ledger).expect("ledger dir created");
    world.cli = CliState {
        workspace: Some(root),
        claimless_dir: Some(claimless),
        ledger_dir: Some(ledger),
        ..CliState::default()
    };
}

#[when("the CLI runs compile on the valid specification directory")]
fn cli_compile_valid(world: &mut Forge) {
    let dir: String = path_arg(&world.cli.valid_dir, "valid spec dir exists");
    world.cli.last_output = Some(run(["compile", &dir]));
}

#[when("the CLI runs compile with JSON output on the valid specification directory")]
fn cli_compile_valid_json(world: &mut Forge) {
    let dir: String = path_arg(&world.cli.valid_dir, "valid spec dir exists");
    world.cli.last_output = Some(run(["compile", &dir, "--json"]));
}

#[when("the CLI runs compile on a defective specification directory")]
fn cli_compile_defective(world: &mut Forge) {
    let dir: String = path_arg(&world.cli.defective_dir, "defective spec dir exists");
    world.cli.last_output = Some(run(["compile", &dir]));
}

#[when("the CLI runs compile with JSON output on a multi-file specification directory")]
fn cli_compile_multi_file_json(world: &mut Forge) {
    let root: &PathBuf = world.cli.workspace.as_ref().expect("workspace exists");
    let multi: PathBuf = root.join("multi");
    fs::create_dir_all(&multi).expect("multi dir created");
    let full: String = fixture_text("sdv-message.md");
    let split_marker: &str = "```specforge\nkind: property\n";
    let (machine_part, claims_rest): (&str, &str) = full.split_once(split_marker).expect("fixture has property block");
    let claims_part: String = format!("# Claims\n\n{split_marker}{claims_rest}");
    fs::write(multi.join("02-claims.md"), &claims_part).expect("claims file written");
    fs::write(multi.join("01-machine.md"), machine_part).expect("machine file written");
    let expected: GraphSnapshot = graph::compile_graph(&[
        Source {
            path: "01-machine.md".to_string(),
            text: machine_part.to_string(),
        },
        Source {
            path: "02-claims.md".to_string(),
            text: claims_part,
        },
    ])
    .expect("sorted multi-file snapshot compiles");
    world.cli.expected_multi_snapshot = Some(expected.snapshot_id.as_str().to_string());
    let dir_arg: String = multi.to_str().expect("utf-8 path").to_string();
    world.cli.last_output = Some(run(["compile", &dir_arg, "--json"]));
}

#[when("the CLI runs compile on a specification directory containing a dangling markdown symlink")]
fn cli_compile_dangling_symlink(world: &mut Forge) {
    let root: &PathBuf = world.cli.workspace.as_ref().expect("workspace exists");
    let dir: PathBuf = write_spec_dir(root, "dangling", "01-message.md", &fixture_text("sdv-message.md"));
    let link_path: PathBuf = dir.join("02-broken.md");
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("missing-target.md"), &link_path).expect("dangling symlink created");
    let dir_arg: String = dir.to_str().expect("utf-8 path").to_string();
    world.cli.last_output = Some(run(["compile", &dir_arg]));
}

#[given(expr = "the CLI verifies property {string} for claim {string} with seed {int} and budget {int} into the ledger as JSON")]
#[when(expr = "the CLI verifies property {string} for claim {string} with seed {int} and budget {int} into the ledger as JSON")]
fn cli_verify_into_ledger_json(world: &mut Forge, property: String, claim: String, seed: u64, budget: usize) {
    let specs: String = active_verify_dir(world);
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    let seed_str: String = seed.to_string();
    let budget_str: String = budget.to_string();
    world.cli.last_output = Some(run([
        "verify",
        &specs,
        "--property",
        &property,
        "--claim",
        &claim,
        "--seed",
        &seed_str,
        "--budget",
        &budget_str,
        "--ledger",
        &ledger,
        "--json",
    ]));
}

#[when(expr = "the CLI verifies property {string} for claim {string} with seed {int} and budget {int} without a ledger")]
fn cli_verify_no_ledger(world: &mut Forge, property: String, claim: String, seed: u64, budget: usize) {
    let specs: String = active_verify_dir(world);
    let seed_str: String = seed.to_string();
    let budget_str: String = budget.to_string();
    world.cli.last_output = Some(run([
        "verify",
        &specs,
        "--property",
        &property,
        "--claim",
        &claim,
        "--seed",
        &seed_str,
        "--budget",
        &budget_str,
    ]));
}

#[when(expr = "the CLI verifies property {string} on the claimless specification directory")]
fn cli_verify_claimless(world: &mut Forge, property: String) {
    let specs: String = path_arg(&world.cli.claimless_dir, "claimless spec dir exists");
    world.cli.last_output = Some(run(["verify", &specs, "--property", &property]));
}

#[when("the CLI assesses the safe specification directory against the ledger")]
fn cli_assess_safe(world: &mut Forge) {
    let specs: String = path_arg(&world.cli.safe_dir, "safe spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["assess", &specs, "--ledger", &ledger]));
}

#[when("the CLI assesses the violating specification directory against the ledger")]
fn cli_assess_violating(world: &mut Forge) {
    let specs: String = path_arg(&world.cli.violating_dir, "violating spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["assess", &specs, "--ledger", &ledger]));
}

#[when("the CLI assesses the property-edit specification directory against the ledger")]
fn cli_assess_property_edit(world: &mut Forge) {
    let specs: String = path_arg(&world.cli.property_edit_dir, "property-edit spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["assess", &specs, "--ledger", &ledger]));
}

#[when("the CLI assesses the claimless specification directory against the ledger")]
fn cli_assess_claimless(world: &mut Forge) {
    let specs: String = path_arg(&world.cli.claimless_dir, "claimless spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["assess", &specs, "--ledger", &ledger]));
}

#[when(expr = "the CLI assesses claim {string} on the safe specification directory against the ledger as JSON")]
fn cli_assess_safe_claim_json(world: &mut Forge, claim: String) {
    let specs: String = path_arg(&world.cli.safe_dir, "safe spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["assess", &specs, "--ledger", &ledger, "--claim", &claim, "--json"]));
}

#[when(expr = "the CLI assesses claim {string} on the claimless specification directory against the ledger as JSON")]
fn cli_assess_claimless_claim_json(world: &mut Forge, claim: String) {
    let specs: String = path_arg(&world.cli.claimless_dir, "claimless spec dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["assess", &specs, "--ledger", &ledger, "--claim", &claim, "--json"]));
}

#[when("the CLI runs ledger verify on the ledger directory")]
#[when("the CLI runs ledger verify on the empty ledger directory")]
fn cli_ledger_verify(world: &mut Forge) {
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["ledger", &ledger, "verify"]));
}

#[when("the CLI runs ledger export on the ledger directory")]
fn cli_ledger_export(world: &mut Forge) {
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["ledger", &ledger, "export"]));
}

#[when("the CLI runs ledger list with JSON output on the ledger directory")]
fn cli_ledger_list_json(world: &mut Forge) {
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["ledger", &ledger, "list", "--json"]));
}

#[when("the CLI runs ledger verify on a nonexistent ledger directory")]
fn cli_ledger_verify_missing(world: &mut Forge) {
    let missing: PathBuf = world.cli.workspace.as_ref().expect("workspace exists").join("does-not-exist");
    let missing_arg: String = missing.to_str().expect("utf-8 path").to_string();
    world.cli.last_output = Some(run(["ledger", &missing_arg, "verify"]));
}

#[when("a persisted ledger blob is tampered on disk")]
fn tamper_ledger_blob(world: &mut Forge) {
    let blobs_dir: PathBuf = world.cli.ledger_dir.as_ref().expect("ledger dir exists").join("blobs");
    let mut entries: Vec<PathBuf> = fs::read_dir(&blobs_dir)
        .expect("blobs dir readable")
        .map(|entry: Result<fs::DirEntry, std::io::Error>| entry.expect("entry").path())
        .collect();
    entries.sort();
    let blob: &PathBuf = entries.first().expect("at least one blob exists");
    fs::write(blob, b"tampered counterexample bytes").expect("blob tampered");
}

#[when("the CLI computes impact from baseline to unrelated-edit against the ledger as JSON")]
fn cli_impact_unrelated(world: &mut Forge) {
    let from: String = path_arg(&world.cli.safe_dir, "baseline dir exists");
    let to: String = path_arg(&world.cli.unrelated_dir, "unrelated dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["impact", "--from", &from, "--to", &to, "--ledger", &ledger, "--json"]));
}

#[when("the CLI computes impact from baseline to property-edit against the ledger as JSON")]
fn cli_impact_property_edit(world: &mut Forge) {
    let from: String = path_arg(&world.cli.safe_dir, "baseline dir exists");
    let to: String = path_arg(&world.cli.property_edit_dir, "property-edit dir exists");
    let ledger: String = path_arg(&world.cli.ledger_dir, "ledger dir exists");
    world.cli.last_output = Some(run(["impact", "--from", &from, "--to", &to, "--ledger", &ledger, "--json"]));
}

#[when(expr = "the CLI runs with raw arguments {string}")]
fn cli_raw_args(world: &mut Forge, raw: String) {
    let tokens: Vec<String> = raw.split_whitespace().map(str::to_string).collect();
    world.cli.last_output = Some(run(tokens));
}
