use std::process::ExitCode;

fn main() -> ExitCode {
    match red_table::run_from(std::env::args_os().skip(1)) {
        Ok(outcome) => ExitCode::from(outcome.exit_code()),
        Err(error) => {
            eprintln!("red-table: {error}");
            ExitCode::FAILURE
        }
    }
}
