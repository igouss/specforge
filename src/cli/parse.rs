use std::path::PathBuf;

pub(crate) const DEFAULT_SEED: u64 = 0;
pub(crate) const DEFAULT_BUDGET: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LedgerAction {
    Verify,
    Export,
    List,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Help,
    Compile {
        specs_dir: PathBuf,
        json: bool,
    },
    Assess {
        specs_dir: PathBuf,
        ledger_dir: PathBuf,
        claim: Option<String>,
        json: bool,
    },
    Verify {
        specs_dir: PathBuf,
        property: String,
        claims: Vec<String>,
        seed: u64,
        budget: usize,
        ledger_dir: Option<PathBuf>,
        json: bool,
    },
    Ledger {
        ledger_dir: PathBuf,
        action: LedgerAction,
        json: bool,
    },
    Impact {
        from_dir: PathBuf,
        to_dir: PathBuf,
        ledger_dir: PathBuf,
        json: bool,
    },
    Argument {
        specs_dir: PathBuf,
        ledger_dir: PathBuf,
        claim: String,
        subclaims: Vec<String>,
        json: bool,
    },
    Recommend {
        specs_dir: PathBuf,
        ledger_dir: PathBuf,
        json: bool,
    },
}

pub(crate) fn usage_text() -> &'static str {
    "Usage:
  specforge compile <specs-dir> [--json]
  specforge assess <specs-dir> --ledger <dir> [--claim <id>] [--json]
  specforge verify <specs-dir> --property <id> [--claim <id>] [--seed <u64>] [--budget <n>] [--ledger <dir>] [--json]
  specforge ledger <dir> {verify|export|list} [--json]
  specforge impact --from <old-specs-dir> --to <new-specs-dir> --ledger <dir> [--json]
  specforge argument <specs-dir> --ledger <dir> --claim <id> [--subclaim <id>...] [--json]
  specforge recommend <specs-dir> --ledger <dir> [--json]"
}

fn split_flag(arg: &str) -> (&str, Option<&str>) {
    match arg.split_once('=') {
        Some((name, value)) => (name, Some(value)),
        None => (arg, None),
    }
}

fn take_value(flag: &str, inline: Option<&str>, args: &[String], index: &mut usize) -> Result<String, String> {
    if let Some(value) = inline {
        if value.is_empty() {
            return Err(format!("flag {flag} requires a non-empty value"));
        }
        return Ok(value.to_string());
    }
    let next_index: usize = *index + 1;
    let Some(value): Option<&String> = args.get(next_index) else {
        return Err(format!("flag {flag} requires a value"));
    };
    if value.is_empty() || value.starts_with("--") {
        return Err(format!("flag {flag} requires a value"));
    }
    *index = next_index;
    Ok(value.clone())
}

fn set_once<T>(slot: &mut Option<T>, value: T, label: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("duplicate {label} specified"));
    }
    *slot = Some(value);
    Ok(())
}

fn is_help_flag(arg: &str) -> bool {
    arg == "--help" || arg == "-h"
}

fn reject_inline_value(flag: &str, inline: Option<&str>) -> Result<(), String> {
    if inline.is_some() {
        return Err(format!("flag {flag} does not accept a value"));
    }
    Ok(())
}

fn parse_compile(args: &[String]) -> Result<Command, String> {
    let mut specs_dir: Option<PathBuf> = None;
    let mut json: bool = false;
    for arg in args {
        if is_help_flag(arg) {
            return Ok(Command::Help);
        }
        let (flag, inline): (&str, Option<&str>) = split_flag(arg);
        if flag == "--json" {
            reject_inline_value(flag, inline)?;
            json = true;
        } else if arg.starts_with('-') {
            return Err(format!("unknown flag for compile: {arg}"));
        } else {
            set_once(&mut specs_dir, PathBuf::from(arg), "specs directory")?;
        }
    }
    let dir: PathBuf = specs_dir.ok_or_else(|| "compile requires <specs-dir>".to_string())?;
    Ok(Command::Compile { specs_dir: dir, json })
}

fn parse_assess(args: &[String]) -> Result<Command, String> {
    let mut specs_dir: Option<PathBuf> = None;
    let mut ledger_dir: Option<PathBuf> = None;
    let mut claim: Option<String> = None;
    let mut json: bool = false;
    let mut index: usize = 0;
    while let Some(arg) = args.get(index) {
        if is_help_flag(arg) {
            return Ok(Command::Help);
        }
        let (flag, inline): (&str, Option<&str>) = split_flag(arg);
        match flag {
            "--json" => {
                reject_inline_value(flag, inline)?;
                json = true;
            }
            "--ledger" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                set_once(&mut ledger_dir, PathBuf::from(value), "flag --ledger")?;
            }
            "--claim" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                set_once(&mut claim, value, "flag --claim")?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag for assess: {arg}"));
            }
            _ => {
                set_once(&mut specs_dir, PathBuf::from(arg), "specs directory")?;
            }
        }
        index += 1;
    }
    let dir: PathBuf = specs_dir.ok_or_else(|| "assess requires <specs-dir>".to_string())?;
    let ledger: PathBuf = ledger_dir.ok_or_else(|| "assess requires --ledger <dir>".to_string())?;
    Ok(Command::Assess {
        specs_dir: dir,
        ledger_dir: ledger,
        claim,
        json,
    })
}

fn parse_verify(args: &[String]) -> Result<Command, String> {
    let mut specs_dir: Option<PathBuf> = None;
    let mut property: Option<String> = None;
    let mut claims: Vec<String> = Vec::new();
    let mut seed: Option<u64> = None;
    let mut budget: Option<usize> = None;
    let mut ledger_dir: Option<PathBuf> = None;
    let mut json: bool = false;
    let mut index: usize = 0;
    while let Some(arg) = args.get(index) {
        if is_help_flag(arg) {
            return Ok(Command::Help);
        }
        let (flag, inline): (&str, Option<&str>) = split_flag(arg);
        match flag {
            "--json" => {
                reject_inline_value(flag, inline)?;
                json = true;
            }
            "--property" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                set_once(&mut property, value, "flag --property")?;
            }
            "--claim" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                if !claims.contains(&value) {
                    claims.push(value);
                }
            }
            "--seed" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                let parsed: u64 = value.parse::<u64>().map_err(|_| format!("invalid --seed value \"{value}\""))?;
                set_once(&mut seed, parsed, "flag --seed")?;
            }
            "--budget" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                let parsed: usize = value.parse::<usize>().map_err(|_| format!("invalid --budget value \"{value}\""))?;
                set_once(&mut budget, parsed, "flag --budget")?;
            }
            "--ledger" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                set_once(&mut ledger_dir, PathBuf::from(value), "flag --ledger")?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag for verify: {arg}"));
            }
            _ => {
                set_once(&mut specs_dir, PathBuf::from(arg), "specs directory")?;
            }
        }
        index += 1;
    }
    let dir: PathBuf = specs_dir.ok_or_else(|| "verify requires <specs-dir>".to_string())?;
    let prop: String = property.ok_or_else(|| "verify requires --property <id>".to_string())?;
    Ok(Command::Verify {
        specs_dir: dir,
        property: prop,
        claims,
        seed: seed.unwrap_or(DEFAULT_SEED),
        budget: budget.unwrap_or(DEFAULT_BUDGET),
        ledger_dir,
        json,
    })
}

fn parse_ledger(args: &[String]) -> Result<Command, String> {
    let mut ledger_dir: Option<PathBuf> = None;
    let mut action: Option<LedgerAction> = None;
    let mut json: bool = false;
    for arg in args {
        if is_help_flag(arg) {
            return Ok(Command::Help);
        }
        let (flag, inline): (&str, Option<&str>) = split_flag(arg);
        if flag == "--json" {
            reject_inline_value(flag, inline)?;
            json = true;
        } else if arg.starts_with('-') {
            return Err(format!("unknown flag for ledger: {arg}"));
        } else if ledger_dir.is_none() {
            ledger_dir = Some(PathBuf::from(arg));
        } else if action.is_none() {
            let parsed: LedgerAction = match arg.as_str() {
                "verify" => LedgerAction::Verify,
                "export" => LedgerAction::Export,
                "list" => LedgerAction::List,
                other => return Err(format!("unknown ledger action \"{other}\"; expected verify, export, or list")),
            };
            action = Some(parsed);
        } else {
            return Err(format!("unexpected extra argument for ledger: {arg}"));
        }
    }
    let dir: PathBuf = ledger_dir.ok_or_else(|| "ledger requires <dir>".to_string())?;
    let selected: LedgerAction = action.ok_or_else(|| "ledger requires an action: verify, export, or list".to_string())?;
    Ok(Command::Ledger {
        ledger_dir: dir,
        action: selected,
        json,
    })
}

fn parse_impact(args: &[String]) -> Result<Command, String> {
    let mut from_dir: Option<PathBuf> = None;
    let mut to_dir: Option<PathBuf> = None;
    let mut ledger_dir: Option<PathBuf> = None;
    let mut json: bool = false;
    let mut index: usize = 0;
    while let Some(arg) = args.get(index) {
        if is_help_flag(arg) {
            return Ok(Command::Help);
        }
        let (flag, inline): (&str, Option<&str>) = split_flag(arg);
        match flag {
            "--json" => {
                reject_inline_value(flag, inline)?;
                json = true;
            }
            "--from" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                set_once(&mut from_dir, PathBuf::from(value), "flag --from")?;
            }
            "--to" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                set_once(&mut to_dir, PathBuf::from(value), "flag --to")?;
            }
            "--ledger" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                set_once(&mut ledger_dir, PathBuf::from(value), "flag --ledger")?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag for impact: {arg}"));
            }
            _ => {
                return Err(format!("unexpected positional argument for impact: {arg}"));
            }
        }
        index += 1;
    }
    let from: PathBuf = from_dir.ok_or_else(|| "impact requires --from <old-specs-dir>".to_string())?;
    let to: PathBuf = to_dir.ok_or_else(|| "impact requires --to <new-specs-dir>".to_string())?;
    let ledger: PathBuf = ledger_dir.ok_or_else(|| "impact requires --ledger <dir>".to_string())?;
    Ok(Command::Impact {
        from_dir: from,
        to_dir: to,
        ledger_dir: ledger,
        json,
    })
}

fn parse_argument(args: &[String]) -> Result<Command, String> {
    let mut specs_dir: Option<PathBuf> = None;
    let mut ledger_dir: Option<PathBuf> = None;
    let mut claim: Option<String> = None;
    let mut subclaims: Vec<String> = Vec::new();
    let mut json: bool = false;
    let mut index: usize = 0;
    while let Some(arg) = args.get(index) {
        if is_help_flag(arg) {
            return Ok(Command::Help);
        }
        let (flag, inline): (&str, Option<&str>) = split_flag(arg);
        match flag {
            "--json" => {
                reject_inline_value(flag, inline)?;
                json = true;
            }
            "--ledger" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                set_once(&mut ledger_dir, PathBuf::from(value), "flag --ledger")?;
            }
            "--claim" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                set_once(&mut claim, value, "flag --claim")?;
            }
            "--subclaim" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                subclaims.push(value);
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag for argument: {arg}"));
            }
            _ => {
                set_once(&mut specs_dir, PathBuf::from(arg), "specs directory")?;
            }
        }
        index += 1;
    }
    let dir: PathBuf = specs_dir.ok_or_else(|| "argument requires <specs-dir>".to_string())?;
    let ledger: PathBuf = ledger_dir.ok_or_else(|| "argument requires --ledger <dir>".to_string())?;
    let claim_id: String = claim.ok_or_else(|| "argument requires --claim <id>".to_string())?;
    Ok(Command::Argument {
        specs_dir: dir,
        ledger_dir: ledger,
        claim: claim_id,
        subclaims,
        json,
    })
}

fn parse_recommend(args: &[String]) -> Result<Command, String> {
    let mut specs_dir: Option<PathBuf> = None;
    let mut ledger_dir: Option<PathBuf> = None;
    let mut json: bool = false;
    let mut index: usize = 0;
    while let Some(arg) = args.get(index) {
        if is_help_flag(arg) {
            return Ok(Command::Help);
        }
        let (flag, inline): (&str, Option<&str>) = split_flag(arg);
        match flag {
            "--json" => {
                reject_inline_value(flag, inline)?;
                json = true;
            }
            "--ledger" => {
                let value: String = take_value(flag, inline, args, &mut index)?;
                set_once(&mut ledger_dir, PathBuf::from(value), "flag --ledger")?;
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown flag for recommend: {arg}"));
            }
            _ => {
                set_once(&mut specs_dir, PathBuf::from(arg), "specs directory")?;
            }
        }
        index += 1;
    }
    let dir: PathBuf = specs_dir.ok_or_else(|| "recommend requires <specs-dir>".to_string())?;
    let ledger: PathBuf = ledger_dir.ok_or_else(|| "recommend requires --ledger <dir>".to_string())?;
    Ok(Command::Recommend {
        specs_dir: dir,
        ledger_dir: ledger,
        json,
    })
}

pub(crate) fn parse_args(args: &[String]) -> Result<Command, String> {
    let Some((first, rest)): Option<(&String, &[String])> = args.split_first() else {
        return Err("missing subcommand".to_string());
    };
    if is_help_flag(first) {
        return Ok(Command::Help);
    }
    match first.as_str() {
        "compile" => parse_compile(rest),
        "assess" => parse_assess(rest),
        "verify" => parse_verify(rest),
        "ledger" => parse_ledger(rest),
        "impact" => parse_impact(rest),
        "argument" => parse_argument(rest),
        "recommend" => parse_recommend(rest),
        other => Err(format!("unknown subcommand \"{other}\"")),
    }
}
