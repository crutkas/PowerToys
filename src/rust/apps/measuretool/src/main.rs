//! MeasureTool application entry point.

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    ExitCode::from(measuretool_app::run(args) as u8)
}
