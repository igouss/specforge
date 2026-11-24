use std::process::ExitCode;

use specforge::cli::{run, CliOutput};

fn main() -> ExitCode {
    let output: CliOutput = run(std::env::args().skip(1));
    print!("{}", output.stdout);
    eprint!("{}", output.stderr);
    ExitCode::from(output.exit_code)
}
