//! MeasureTool (Screen Ruler) application library.
//!
//! This is the standalone application that performs:
//! - Screen capture via BitBlt
//! - D2D overlay rendering with measurement lines
//! - DirectWrite text for dimension labels
//! - Keyboard input (Esc to exit, mode switching)
//!
//! The module interface DLL launches this as a separate process.

use measuretool_core::settings::MeasureToolSettings;
use measuretool_core::types::MeasureMode;

/// Parse command line arguments.
struct Args {
    /// PID of the parent runner process to watch.
    wait_pid: Option<u32>,
    /// Initial measurement mode.
    mode: MeasureMode,
}

impl Args {
    fn parse(args: Vec<String>) -> Self {
        let mut wait_pid = None;
        let mut mode = MeasureMode::Cross;
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--wait-pid" => {
                    if i + 1 < args.len() {
                        wait_pid = args[i + 1].parse().ok();
                        i += 1;
                    }
                }
                "--mode" => {
                    if i + 1 < args.len() {
                        mode = match args[i + 1].as_str() {
                            "horizontal" => MeasureMode::Horizontal,
                            "vertical" => MeasureMode::Vertical,
                            "bounds" => MeasureMode::Bounds,
                            _ => MeasureMode::Cross,
                        };
                        i += 1;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        Self { wait_pid, mode }
    }
}

/// Main application entry point. Returns 0 on success.
pub fn run(args: Vec<String>) -> i32 {
    let parsed = Args::parse(args);
    let settings = MeasureToolSettings::default();

    eprintln!(
        "[MeasureTool] Starting mode={:?}, tolerance={}, wait_pid={:?}",
        parsed.mode, settings.pixel_tolerance, parsed.wait_pid
    );

    // TODO: Implement the Win32 overlay window, D2D rendering, and screen capture.
    // The overlay loop will:
    // 1. Create a transparent layered window covering all monitors
    // 2. Capture the screen via BitBlt into a PixelBuffer
    // 3. On mouse move: run edge detection + compute measurements
    // 4. Render measurement lines and text via D2D/DirectWrite
    // 5. Handle keyboard: Esc to exit, scroll wheel to adjust tolerance
    //
    // For now, this is a placeholder that exits immediately.
    // The core logic is fully implemented and tested in measuretool-core.

    eprintln!("[MeasureTool] Exiting (overlay not yet implemented)");
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arg_parsing_defaults() {
        let args = Args::parse(vec!["measuretool".to_string()]);
        assert_eq!(args.mode, MeasureMode::Cross);
        assert!(args.wait_pid.is_none());
    }

    #[test]
    fn test_arg_parsing_wait_pid() {
        let args = Args::parse(vec![
            "measuretool".to_string(),
            "--wait-pid".to_string(),
            "1234".to_string(),
        ]);
        assert_eq!(args.wait_pid, Some(1234));
    }

    #[test]
    fn test_arg_parsing_mode() {
        let args = Args::parse(vec![
            "measuretool".to_string(),
            "--mode".to_string(),
            "horizontal".to_string(),
        ]);
        assert_eq!(args.mode, MeasureMode::Horizontal);

        let args = Args::parse(vec![
            "measuretool".to_string(),
            "--mode".to_string(),
            "vertical".to_string(),
        ]);
        assert_eq!(args.mode, MeasureMode::Vertical);

        let args = Args::parse(vec![
            "measuretool".to_string(),
            "--mode".to_string(),
            "bounds".to_string(),
        ]);
        assert_eq!(args.mode, MeasureMode::Bounds);
    }

    #[test]
    fn test_arg_parsing_combined() {
        let args = Args::parse(vec![
            "measuretool".to_string(),
            "--wait-pid".to_string(),
            "5678".to_string(),
            "--mode".to_string(),
            "bounds".to_string(),
        ]);
        assert_eq!(args.wait_pid, Some(5678));
        assert_eq!(args.mode, MeasureMode::Bounds);
    }

    #[test]
    fn test_run_returns_zero() {
        let result = run(vec!["measuretool".to_string()]);
        assert_eq!(result, 0);
    }
}
