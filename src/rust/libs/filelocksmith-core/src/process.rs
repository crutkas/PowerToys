use crate::handle_info::ProcessResult;
use serde_json;

/// Format process results as human-readable text.
pub fn format_text(results: &[ProcessResult]) -> String {
    if results.is_empty() {
        return "No processes found locking the specified file(s).\n".to_string();
    }

    let mut out = String::new();
    for pr in results {
        out.push_str(&format!(
            "[{}] {} (User: {})\n",
            pr.pid, pr.name, pr.user
        ));
        for f in &pr.files {
            out.push_str(&format!("  {f}\n"));
        }
    }
    out
}

/// Format process results as JSON.
pub fn format_json(results: &[ProcessResult]) -> String {
    serde_json::to_string_pretty(results).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handle_info::ProcessResult;

    fn sample() -> Vec<ProcessResult> {
        vec![
            ProcessResult {
                name: "notepad.exe".into(),
                pid: 1234,
                user: "DESKTOP\\Alice".into(),
                files: vec![r"C:\file.txt".into()],
            },
            ProcessResult {
                name: "explorer.exe".into(),
                pid: 5678,
                user: "DESKTOP\\Bob".into(),
                files: vec![r"C:\dir\a.dll".into(), r"C:\dir\b.dll".into()],
            },
        ]
    }

    #[test]
    fn text_format_contains_pid_and_name() {
        let text = format_text(&sample());
        assert!(text.contains("[1234] notepad.exe"));
        assert!(text.contains("[5678] explorer.exe"));
    }

    #[test]
    fn text_format_contains_files() {
        let text = format_text(&sample());
        assert!(text.contains(r"C:\file.txt"));
        assert!(text.contains(r"C:\dir\a.dll"));
    }

    #[test]
    fn text_format_empty() {
        let text = format_text(&[]);
        assert!(text.contains("No processes found"));
    }

    #[test]
    fn json_format_parses() {
        let json = format_json(&sample());
        let parsed: Vec<ProcessResult> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].pid, 1234);
    }

    #[test]
    fn json_format_contains_user() {
        let json = format_json(&sample());
        assert!(json.contains("DESKTOP\\\\Alice"));
    }
}
