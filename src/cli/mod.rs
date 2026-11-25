mod commands;
mod format;
mod parse;
mod recommend;

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::graph::canonical_json;
use parse::{parse_args, usage_text, Command};

pub const EXIT_OK: u8 = 0;
pub const EXIT_CHECK_FAILED: u8 = 1;
pub const EXIT_ERROR: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    pub exit_code: u8,
    pub stdout: String,
    pub stderr: String,
}

impl CliOutput {
    pub(crate) fn ok(stdout: String) -> CliOutput {
        CliOutput {
            exit_code: EXIT_OK,
            stdout: with_trailing_newline(stdout),
            stderr: String::new(),
        }
    }

    pub(crate) fn with_status(exit_code: u8, stdout: String) -> CliOutput {
        CliOutput {
            exit_code,
            stdout: with_trailing_newline(stdout),
            stderr: String::new(),
        }
    }

    pub(crate) fn error(message: String, json: bool) -> CliOutput {
        let stderr: String = with_trailing_newline(message.clone());
        let stdout: String = if json {
            let payload: ErrorPayload<'_> = ErrorPayload {
                status: "error",
                error: &message,
            };
            match canonical_json(&payload) {
                Ok(encoded) => with_trailing_newline(encoded),
                Err(_) => String::new(),
            }
        } else {
            String::new()
        };
        CliOutput {
            exit_code: EXIT_ERROR,
            stdout,
            stderr,
        }
    }

    pub(crate) fn defects(defects: Vec<String>, json: bool) -> CliOutput {
        let stderr: String = with_trailing_newline(defects.join("\n"));
        let stdout: String = if json {
            let payload: DefectPayload<'_> = DefectPayload {
                status: "defective",
                defects: &defects,
            };
            match canonical_json(&payload) {
                Ok(encoded) => with_trailing_newline(encoded),
                Err(_) => String::new(),
            }
        } else {
            String::new()
        };
        CliOutput {
            exit_code: EXIT_ERROR,
            stdout,
            stderr,
        }
    }
}

#[derive(Serialize)]
struct ErrorPayload<'a> {
    status: &'a str,
    error: &'a str,
}

#[derive(Serialize)]
struct DefectPayload<'a> {
    status: &'a str,
    defects: &'a [String],
}

fn with_trailing_newline(mut text: String) -> String {
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

pub(crate) fn utc_timestamp_now() -> String {
    let seconds: u64 = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    };
    let days: i64 = (seconds / 86_400) as i64;
    let time_of_day: u64 = seconds % 86_400;
    let hour: u64 = time_of_day / 3_600;
    let minute: u64 = (time_of_day % 3_600) / 60;
    let second: u64 = time_of_day % 60;
    let z: i64 = days + 719_468;
    let era: i64 = z.div_euclid(146_097);
    let doe: u64 = z.rem_euclid(146_097) as u64;
    let yoe: u64 = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y: i64 = yoe as i64 + era * 400;
    let doy: u64 = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp: u64 = (5 * doy + 2) / 153;
    let day: u64 = doy - (153 * mp + 2) / 5 + 1;
    let month: u64 = if mp < 10 { mp + 3 } else { mp - 9 };
    let year: i64 = y + if month <= 2 { 1 } else { 0 };
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// When CLI arguments are supplied, `run` shall parse and execute the requested subcommand and return its exit code, stdout, and stderr as SF-015 describes.
pub fn run<I, S>(args: I) -> CliOutput
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let raw: Vec<String> = args.into_iter().map(Into::into).collect();
    let command: Command = match parse_args(&raw) {
        Ok(command) => command,
        Err(message) => return CliOutput::error(format!("{message}\n\n{}", usage_text()), false),
    };
    match command {
        Command::Help => CliOutput::ok(usage_text().to_string()),
        Command::Compile { specs_dir, json } => commands::run_compile(&specs_dir, json),
        Command::Assess {
            specs_dir,
            ledger_dir,
            claim,
            json,
        } => commands::run_assess(&specs_dir, &ledger_dir, claim.as_deref(), json),
        Command::Verify {
            specs_dir,
            property,
            claims,
            seed,
            budget,
            ledger_dir,
            json,
        } => commands::run_verify(&specs_dir, &property, &claims, seed, budget, ledger_dir.as_deref(), json),
        Command::Ledger { ledger_dir, action, json } => commands::run_ledger(&ledger_dir, action, json),
        Command::Impact {
            from_dir,
            to_dir,
            ledger_dir,
            json,
        } => commands::run_impact(&from_dir, &to_dir, &ledger_dir, json),
        Command::Argument {
            specs_dir,
            ledger_dir,
            claim,
            subclaims,
            json,
        } => commands::run_argument(&specs_dir, &ledger_dir, &claim, &subclaims, json),
        Command::Recommend { specs_dir, ledger_dir, json } => commands::run_recommend(&specs_dir, &ledger_dir, json),
    }
}
