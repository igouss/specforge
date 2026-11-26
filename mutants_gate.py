#!/usr/bin/env python3
"""SpecForge mutation gate.

Fails if:
  1. Any surviving mutant (missed.txt) is not listed in mutants-equivalents.txt,
     or any mutant timed out (timeout.txt).
  2. Any entry in mutants-equivalents.txt is stale (appears in caught.txt or
     unviable.txt).
  3. Zero mutants were tested (unless --allow-empty is passed), or the
     equivalents file contains duplicate or reasonless entries, or cargo-mutants
     failed its baseline/build.
"""

from __future__ import annotations

import json
import pathlib
import shutil
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent
DEFAULT_EQUIVALENTS = ROOT / "mutants-equivalents.txt"
DEFAULT_WORK_DIR = ROOT / "target" / "mutants-gate"
OUTCOME_MARKERS = ("outcomes.json", "caught.txt", "missed.txt", "timeout.txt", "unviable.txt")

USAGE = """Usage:
  python3 mutants_gate.py [GATE_OPTIONS] [CARGO_MUTANTS_ARGS...]
  python3 mutants_gate.py --out-dir <DIR> [--equivalents <FILE>] [--allow-empty]

Gate options:
  --equivalents <FILE>  Path to accepted equivalents list
                        (default: mutants-equivalents.txt)
  --out-dir <DIR>       Evaluate an existing cargo-mutants output directory
                        (either <DIR> or <DIR>/mutants.out) without running
                        cargo mutants
  --work-dir <DIR>      Output parent directory for `cargo mutants -o <DIR>`
                        (default: target/mutants-gate)
  --allow-empty         Allow exit 0 when 0 mutants were tested (e.g. for
                        --in-diff runs touching no Rust source files)
  -h, --help            Show this help message and exit

Run mode (when --out-dir is omitted):
  Invokes `cargo mutants --all-features --no-shuffle -o <work-dir> ...`
  passing through any extra arguments (such as `-f src/view.rs` or
  `--in-diff target/mutants-gate/diff.patch`), then evaluates
  `<work-dir>/mutants.out`.
"""


def parse_args(argv: list[str]) -> tuple[pathlib.Path, pathlib.Path | None, pathlib.Path, bool, list[str]]:
    equivalents = DEFAULT_EQUIVALENTS
    out_dir: pathlib.Path | None = None
    work_dir = DEFAULT_WORK_DIR
    work_dir_explicit = False
    allow_empty = False
    cargo_args: list[str] = []

    i = 0
    while i < len(argv):
        arg = argv[i]
        if arg == "--":
            cargo_args.extend(argv[i:])
            break
        if arg in ("-h", "--help"):
            print(USAGE, end="")
            sys.exit(0)
        if arg == "--allow-empty":
            allow_empty = True
            i += 1
        elif arg == "--equivalents":
            if i + 1 >= len(argv):
                raise ValueError("--equivalents requires a file path")
            equivalents = pathlib.Path(argv[i + 1])
            i += 2
        elif arg.startswith("--equivalents="):
            equivalents = pathlib.Path(arg.split("=", 1)[1])
            i += 1
        elif arg == "--out-dir":
            if i + 1 >= len(argv):
                raise ValueError("--out-dir requires a directory path")
            out_dir = pathlib.Path(argv[i + 1])
            i += 2
        elif arg.startswith("--out-dir="):
            out_dir = pathlib.Path(arg.split("=", 1)[1])
            i += 1
        elif arg == "--work-dir":
            if i + 1 >= len(argv):
                raise ValueError("--work-dir requires a directory path")
            work_dir = pathlib.Path(argv[i + 1])
            work_dir_explicit = True
            i += 2
        elif arg.startswith("--work-dir="):
            work_dir = pathlib.Path(arg.split("=", 1)[1])
            work_dir_explicit = True
            i += 1
        else:
            cargo_args.append(arg)
            i += 1

    if out_dir is not None and cargo_args:
        raise ValueError("--out-dir evaluates an existing directory and cannot be combined with cargo mutants arguments")
    if out_dir is not None and work_dir_explicit:
        raise ValueError("--out-dir and --work-dir are mutually exclusive")

    before_dd = cargo_args[: cargo_args.index("--")] if "--" in cargo_args else cargo_args
    for token in before_dd:
        if token in ("-o", "--output") or token.startswith("--output=") or (token.startswith("-o") and len(token) > 2):
            raise ValueError("pass --work-dir instead of -o/--output so mutants_gate.py controls the output directory")
        if token in ("--list", "--list-files"):
            raise ValueError(f"{token} does not execute mutation tests and cannot satisfy the gate")

    return equivalents, out_dir, work_dir, allow_empty, cargo_args


def load_equivalents(path: pathlib.Path) -> dict[str, str]:
    if not path.is_file():
        raise ValueError(f"equivalents file not found: {path}")

    entries: dict[str, str] = {}
    first_line: dict[str, int] = {}
    errors: list[str] = []

    for lineno, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        stripped = raw.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if " | " not in stripped:
            errors.append(f"{path}:{lineno}: missing ' | <reason>' separator: {stripped}")
            continue
        mutant, reason = stripped.rsplit(" | ", 1)
        mutant = mutant.strip()
        reason = reason.strip()
        if not mutant:
            errors.append(f"{path}:{lineno}: empty mutant signature before '|': {stripped}")
            continue
        if not reason:
            errors.append(f"{path}:{lineno}: empty reason after '|': {stripped}")
            continue
        if mutant.endswith((" replace", " with")):
            errors.append(f"{path}:{lineno}: missing ' | <reason>' separator (found '|' inside mutant): {stripped}")
            continue
        if ":" not in mutant:
            errors.append(f"{path}:{lineno}: invalid mutant signature (expected '<path>:<line>:<col>: ...'): {stripped}")
            continue
        if mutant in entries:
            errors.append(
                f"{path}:{lineno}: duplicate equivalent entry (first declared at line {first_line[mutant]}): {mutant}"
            )
            continue
        entries[mutant] = reason
        first_line[mutant] = lineno

    if errors:
        raise ValueError("invalid equivalents file:\n" + "\n".join(f"  - {e}" for e in errors))

    return entries


def has_outcome_files(directory: pathlib.Path) -> bool:
    return directory.is_dir() and any((directory / marker).is_file() for marker in OUTCOME_MARKERS)


def resolve_out_dir(candidate: pathlib.Path) -> pathlib.Path:
    if has_outcome_files(candidate):
        return candidate
    nested = candidate / "mutants.out"
    if has_outcome_files(nested):
        return nested
    raise ValueError(f"no cargo-mutants output found in {candidate} (or {nested})")


def read_outcome_lines(path: pathlib.Path) -> list[str]:
    if not path.is_file():
        return []
    return [line.strip() for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def check_baseline_outcome(mutants_out: pathlib.Path) -> str | None:
    outcomes_path = mutants_out / "outcomes.json"
    if not outcomes_path.is_file():
        return None
    try:
        data = json.loads(outcomes_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return f"could not parse {outcomes_path}: {exc}"
    outcomes = data.get("outcomes")
    if not isinstance(outcomes, list):
        return None
    for entry in outcomes:
        if not isinstance(entry, dict):
            continue
        if entry.get("scenario") == "Baseline":
            summary = entry.get("summary")
            if summary != "Success":
                return f"baseline scenario did not succeed (summary: {summary!r})"
    return None


def run_cargo_mutants(work_dir: pathlib.Path, cargo_args: list[str]) -> tuple[int, pathlib.Path]:
    mutants_out = work_dir / "mutants.out"
    if mutants_out.exists():
        shutil.rmtree(mutants_out)
    work_dir.mkdir(parents=True, exist_ok=True)

    before_dd = cargo_args[: cargo_args.index("--")] if "--" in cargo_args else cargo_args
    cmd: list[str] = ["cargo", "mutants"]
    if "--all-features" not in before_dd and "--features" not in before_dd and "--no-default-features" not in before_dd:
        cmd.append("--all-features")
    if "--no-shuffle" not in before_dd and "--shuffle" not in before_dd:
        cmd.append("--no-shuffle")
    cmd.extend(["-o", str(work_dir)])
    cmd.extend(cargo_args)

    proc = subprocess.run(cmd, cwd=ROOT)
    return proc.returncode, mutants_out


def evaluate(
    mutants_out: pathlib.Path,
    equivalents: dict[str, str],
    equivalents_path: pathlib.Path,
    allow_empty: bool,
) -> int:
    baseline_err = check_baseline_outcome(mutants_out)
    if baseline_err is not None:
        print(f"mutants-gate: FAIL ({baseline_err})", file=sys.stderr)
        return 1

    missed = read_outcome_lines(mutants_out / "missed.txt")
    caught = read_outcome_lines(mutants_out / "caught.txt")
    timeouts = read_outcome_lines(mutants_out / "timeout.txt")
    unviable = read_outcome_lines(mutants_out / "unviable.txt")

    caught_set = set(caught)
    unviable_set = set(unviable)

    unaccepted_missed = [m for m in missed if m not in equivalents]
    accepted_missed = [m for m in missed if m in equivalents]
    stale_entries: list[tuple[str, str]] = [
        (m, "caught") for m in equivalents if m in caught_set
    ] + [
        (m, "unviable") for m in equivalents if m in unviable_set
    ]

    total_tested = len(caught) + len(unviable) + len(missed) + len(timeouts)
    failure_reasons: list[str] = []

    if unaccepted_missed:
        print(f"unaccepted surviving mutants ({len(unaccepted_missed)}):", file=sys.stderr)
        for mutant in unaccepted_missed:
            print(f"  - {mutant}", file=sys.stderr)
        failure_reasons.append(f"{len(unaccepted_missed)} unaccepted survivor(s)")

    if timeouts:
        print(f"timed-out mutants ({len(timeouts)}):", file=sys.stderr)
        for mutant in timeouts:
            print(f"  - {mutant}", file=sys.stderr)
        failure_reasons.append(f"{len(timeouts)} timeout(s)")

    if stale_entries:
        print(
            f"stale equivalent entries in {equivalents_path} ({len(stale_entries)}):",
            file=sys.stderr,
        )
        for mutant, outcome in stale_entries:
            print(f"  - {mutant} ({outcome}; listed reason: {equivalents[mutant]})", file=sys.stderr)
        failure_reasons.append(f"{len(stale_entries)} stale equivalent(s)")

    if total_tested == 0 and not allow_empty:
        failure_reasons.append("0 mutants tested (pass --allow-empty if intentional)")

    counts = (
        f"{len(caught)} caught, {len(unviable)} unviable, "
        f"{len(accepted_missed)} accepted equivalents, "
        f"{len(unaccepted_missed)} unaccepted survivors, "
        f"{len(timeouts)} timeouts, {len(stale_entries)} stale equivalents"
    )

    if failure_reasons:
        print(f"mutants-gate: FAIL ({'; '.join(failure_reasons)}; {counts})", file=sys.stderr)
        return 1

    print(f"mutants-gate: PASS ({counts})")
    return 0


def main(argv: list[str] | None = None) -> int:
    try:
        equivalents_path, out_dir, work_dir, allow_empty, cargo_args = parse_args(
            sys.argv[1:] if argv is None else argv
        )
        equivalents = load_equivalents(equivalents_path)
    except ValueError as exc:
        print(f"mutants-gate: ERROR: {exc}", file=sys.stderr)
        return 1

    if out_dir is not None:
        try:
            mutants_out = resolve_out_dir(out_dir)
        except ValueError as exc:
            print(f"mutants-gate: FAIL ({exc})", file=sys.stderr)
            return 1
        return evaluate(mutants_out, equivalents, equivalents_path, allow_empty)

    returncode, mutants_out = run_cargo_mutants(work_dir, cargo_args)
    if returncode not in (0, 2, 3):
        print(
            f"mutants-gate: FAIL (cargo mutants exited with code {returncode}; baseline tests or build failed)",
            file=sys.stderr,
        )
        return 1

    if not has_outcome_files(mutants_out):
        if returncode == 0 and allow_empty:
            print("mutants-gate: PASS (0 mutants tested; --allow-empty)")
            return 0
        print(
            f"mutants-gate: FAIL (0 mutants tested; cargo mutants produced no output at {mutants_out}; "
            "pass --allow-empty if this run intentionally matched no Rust code)",
            file=sys.stderr,
        )
        return 1

    return evaluate(mutants_out, equivalents, equivalents_path, allow_empty)


if __name__ == "__main__":
    sys.exit(main())
