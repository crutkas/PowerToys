use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tracing_appender::rolling::{RollingFileAppender, Rotation};

// ---------------------------------------------------------------------------
// Log levels
// ---------------------------------------------------------------------------

/// Log severity levels matching the C++ PowerToys logger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    /// Map to the equivalent `tracing::Level` for interop.
    pub fn to_tracing_level(&self) -> tracing::Level {
        match self {
            Self::Trace => tracing::Level::TRACE,
            Self::Debug => tracing::Level::DEBUG,
            Self::Info => tracing::Level::INFO,
            Self::Warn => tracing::Level::WARN,
            Self::Error => tracing::Level::ERROR,
        }
    }
}

// ---------------------------------------------------------------------------
// Logger (standalone, testable instance)
// ---------------------------------------------------------------------------

/// A file-backed rotating logger.
///
/// Log files are created under `<internal_path>/<module_name>/` with daily
/// rotation and a maximum of 5 retained files.
pub struct Logger {
    appender: RollingFileAppender,
    log_dir: PathBuf,
}

impl Logger {
    /// Create a new logger.
    ///
    /// * `module_name`   – sub-directory name (e.g. `"Awake"`).
    /// * `internal_path` – base directory (e.g. `%LOCALAPPDATA%\Microsoft\PowerToys\v0.80`).
    /// * `log_name`      – file-name prefix (e.g. `"awake-log"`).
    pub fn new(module_name: &str, internal_path: &str, log_name: &str) -> std::io::Result<Self> {
        let log_dir = Path::new(internal_path).join(module_name);
        std::fs::create_dir_all(&log_dir)?;

        let appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix(log_name)
            .filename_suffix("log")
            .max_log_files(5)
            .build(&log_dir)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(Self { appender, log_dir })
    }

    /// Write a single log line: `[timestamp] [LEVEL] message\n`.
    pub fn log(&mut self, level: LogLevel, message: &str) {
        let ts = format_timestamp();
        let line = format!("[{ts}] [{}] {message}\n", level.as_str());
        let _ = self.appender.write_all(line.as_bytes());
    }

    pub fn trace(&mut self, msg: &str) {
        self.log(LogLevel::Trace, msg);
    }
    pub fn debug(&mut self, msg: &str) {
        self.log(LogLevel::Debug, msg);
    }
    pub fn info(&mut self, msg: &str) {
        self.log(LogLevel::Info, msg);
    }
    pub fn warn(&mut self, msg: &str) {
        self.log(LogLevel::Warn, msg);
    }
    pub fn error(&mut self, msg: &str) {
        self.log(LogLevel::Error, msg);
    }

    pub fn flush(&mut self) {
        let _ = self.appender.flush();
    }

    /// Directory where log files are written.
    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }
}

// ---------------------------------------------------------------------------
// Global logger (used by the C FFI layer)
// ---------------------------------------------------------------------------

static GLOBAL_LOGGER: Mutex<Option<Logger>> = Mutex::new(None);

/// Initialise (or re-initialise) the global logger.
/// Returns the log directory on success.
pub fn init_logger(
    module_name: &str,
    internal_path: &str,
    log_name: &str,
) -> std::io::Result<PathBuf> {
    let logger = Logger::new(module_name, internal_path, log_name)?;
    let dir = logger.log_dir().to_path_buf();
    *GLOBAL_LOGGER.lock().unwrap() = Some(logger);
    Ok(dir)
}

fn log_global(level: LogLevel, message: &str) {
    if let Some(lg) = GLOBAL_LOGGER.lock().unwrap().as_mut() {
        lg.log(level, message);
    }
}

pub fn log_trace(msg: &str) {
    log_global(LogLevel::Trace, msg);
}
pub fn log_debug(msg: &str) {
    log_global(LogLevel::Debug, msg);
}
pub fn log_info(msg: &str) {
    log_global(LogLevel::Info, msg);
}
pub fn log_warn(msg: &str) {
    log_global(LogLevel::Warn, msg);
}
pub fn log_error(msg: &str) {
    log_global(LogLevel::Error, msg);
}

pub fn flush() {
    if let Some(lg) = GLOBAL_LOGGER.lock().unwrap().as_mut() {
        lg.flush();
    }
}

// ---------------------------------------------------------------------------
// Timestamp formatting
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn format_timestamp() -> String {
    #[repr(C)]
    struct SYSTEMTIME {
        w_year: u16,
        w_month: u16,
        w_day_of_week: u16,
        w_day: u16,
        w_hour: u16,
        w_minute: u16,
        w_second: u16,
        w_milliseconds: u16,
    }

    unsafe extern "system" {
        unsafe fn GetLocalTime(lp: *mut SYSTEMTIME);
    }

    let mut st = SYSTEMTIME {
        w_year: 0,
        w_month: 0,
        w_day_of_week: 0,
        w_day: 0,
        w_hour: 0,
        w_minute: 0,
        w_second: 0,
        w_milliseconds: 0,
    };
    unsafe {
        GetLocalTime(&mut st);
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:03}",
        st.w_year, st.w_month, st.w_day, st.w_hour, st.w_minute, st.w_second, st.w_milliseconds,
    )
}

#[cfg(not(windows))]
fn format_timestamp() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let millis = dur.subsec_millis();

    let days = (secs / 86400) as i64;
    let tod = (secs % 86400) as u32;
    let (h, mi, s) = (tod / 3600, (tod % 3600) / 60, tod % 60);

    // Howard Hinnant's civil_from_days (UTC)
    let z = days + 719_468;
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };

    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}.{millis:03}")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read concatenated content of every `.log` file in `dir`.
pub fn read_log_content(dir: &Path) -> std::io::Result<String> {
    let mut content = String::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("log") {
            content.push_str(&std::fs::read_to_string(&path)?);
        }
    }
    Ok(content)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    /// Serialises tests that touch the global logger.
    static FFI_LOCK: StdMutex<()> = StdMutex::new(());

    /// Create a fresh, empty test directory under `<crate>/test_logs/<name>`.
    fn test_dir(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_logs")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn read_logs(dir: &Path) -> String {
        read_log_content(dir).unwrap_or_default()
    }

    /// Encode `&str` → null-terminated UTF-16 for FFI tests.
    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    // ── 1 ────────────────────────────────────────────────────────────
    #[test]
    fn test_init_creates_log_directory() {
        let base = test_dir("init_creates_dir");
        let logger = Logger::new("mymodule", base.to_str().unwrap(), "app").unwrap();
        assert!(logger.log_dir().exists());
        assert!(logger.log_dir().ends_with("mymodule"));
    }

    // ── 2 ────────────────────────────────────────────────────────────
    #[test]
    fn test_log_trace_writes_to_file() {
        let base = test_dir("trace_writes");
        let mut lg = Logger::new("m", base.to_str().unwrap(), "app").unwrap();
        lg.trace("trace_message");
        lg.flush();
        let c = read_logs(lg.log_dir());
        assert!(c.contains("trace_message"));
        assert!(c.contains("[TRACE]"));
    }

    // ── 3 ────────────────────────────────────────────────────────────
    #[test]
    fn test_log_debug_writes_to_file() {
        let base = test_dir("debug_writes");
        let mut lg = Logger::new("m", base.to_str().unwrap(), "app").unwrap();
        lg.debug("debug_message");
        lg.flush();
        let c = read_logs(lg.log_dir());
        assert!(c.contains("debug_message"));
        assert!(c.contains("[DEBUG]"));
    }

    // ── 4 ────────────────────────────────────────────────────────────
    #[test]
    fn test_log_info_writes_to_file() {
        let base = test_dir("info_writes");
        let mut lg = Logger::new("m", base.to_str().unwrap(), "app").unwrap();
        lg.info("info_message");
        lg.flush();
        let c = read_logs(lg.log_dir());
        assert!(c.contains("info_message"));
        assert!(c.contains("[INFO]"));
    }

    // ── 5 ────────────────────────────────────────────────────────────
    #[test]
    fn test_log_warn_writes_to_file() {
        let base = test_dir("warn_writes");
        let mut lg = Logger::new("m", base.to_str().unwrap(), "app").unwrap();
        lg.warn("warn_message");
        lg.flush();
        let c = read_logs(lg.log_dir());
        assert!(c.contains("warn_message"));
        assert!(c.contains("[WARN]"));
    }

    // ── 6 ────────────────────────────────────────────────────────────
    #[test]
    fn test_log_error_writes_to_file() {
        let base = test_dir("error_writes");
        let mut lg = Logger::new("m", base.to_str().unwrap(), "app").unwrap();
        lg.error("error_message");
        lg.flush();
        let c = read_logs(lg.log_dir());
        assert!(c.contains("error_message"));
        assert!(c.contains("[ERROR]"));
    }

    // ── 7 ────────────────────────────────────────────────────────────
    #[test]
    fn test_log_format_has_timestamp() {
        let base = test_dir("format_ts");
        let mut lg = Logger::new("m", base.to_str().unwrap(), "app").unwrap();
        lg.info("ts_check");
        lg.flush();

        let content = read_logs(lg.log_dir());
        let line = content.lines().next().expect("expected at least one line");

        // [YYYY-MM-DD HH:MM:SS.mmm] …
        assert!(line.starts_with('['), "should start with '['");
        assert_eq!(&line[5..6], "-", "year-month separator");
        assert_eq!(&line[8..9], "-", "month-day separator");
        assert_eq!(&line[11..12], " ", "date-time separator");
        assert_eq!(&line[14..15], ":", "hour-minute separator");
        assert_eq!(&line[17..18], ":", "minute-second separator");
        assert_eq!(&line[20..21], ".", "second-ms separator");
        assert_eq!(&line[24..25], "]", "timestamp closing bracket");
    }

    // ── 8 ────────────────────────────────────────────────────────────
    #[test]
    fn test_log_format_level_in_output() {
        let base = test_dir("format_level");
        let mut lg = Logger::new("m", base.to_str().unwrap(), "app").unwrap();
        lg.warn("level_check");
        lg.flush();
        assert!(read_logs(lg.log_dir()).contains("[WARN]"));
    }

    // ── 9 ────────────────────────────────────────────────────────────
    #[test]
    fn test_log_format_message_preserved() {
        let base = test_dir("format_msg");
        let mut lg = Logger::new("m", base.to_str().unwrap(), "app").unwrap();
        let msg = "exact message content 12345!@#";
        lg.info(msg);
        lg.flush();
        assert!(read_logs(lg.log_dir()).contains(msg));
    }

    // ── 10 ───────────────────────────────────────────────────────────
    #[test]
    fn test_module_name_in_path() {
        let base = test_dir("module_path");
        let lg = Logger::new("fancy-module", base.to_str().unwrap(), "app").unwrap();
        let path_str = lg.log_dir().to_str().unwrap().to_string();
        assert!(
            path_str.contains("fancy-module"),
            "path should contain module name: {path_str}"
        );
    }

    // ── 11 ───────────────────────────────────────────────────────────
    #[test]
    fn test_flush_writes_pending() {
        let base = test_dir("flush_pending");
        let mut lg = Logger::new("m", base.to_str().unwrap(), "app").unwrap();
        lg.info("before_flush");
        lg.flush();
        assert!(read_logs(lg.log_dir()).contains("before_flush"));
    }

    // ── 12 ───────────────────────────────────────────────────────────
    #[test]
    fn test_multiple_inits_different_modules() {
        let base = test_dir("multi_init");
        let mut la = Logger::new("mod_a", base.to_str().unwrap(), "app").unwrap();
        let mut lb = Logger::new("mod_b", base.to_str().unwrap(), "app").unwrap();

        la.info("message_a");
        lb.info("message_b");
        la.flush();
        lb.flush();

        let ca = read_logs(la.log_dir());
        let cb = read_logs(lb.log_dir());

        assert!(ca.contains("message_a"));
        assert!(!ca.contains("message_b"));
        assert!(cb.contains("message_b"));
        assert!(!cb.contains("message_a"));
    }

    // ── 13 ───────────────────────────────────────────────────────────
    #[test]
    fn test_log_rotation_file_naming() {
        let base = test_dir("rotation");
        let mut lg = Logger::new("m", base.to_str().unwrap(), "mylog").unwrap();
        lg.info("rotation_test");
        lg.flush();

        let files: Vec<_> = std::fs::read_dir(lg.log_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("log"))
            .collect();
        assert!(!files.is_empty(), "expected at least one log file");

        let name = files[0].file_name();
        let name = name.to_str().unwrap();
        assert!(
            name.starts_with("mylog"),
            "file should start with prefix: {name}"
        );
        assert!(name.ends_with(".log"), "file should end with .log: {name}");
    }

    // ── 14 ───────────────────────────────────────────────────────────
    #[test]
    fn test_ffi_init_log_flush_cycle() {
        let _lock = FFI_LOCK.lock().unwrap();
        let base = test_dir("ffi_cycle");

        let base_w = to_wide(base.to_str().unwrap());
        let module_w = to_wide("ffimod");
        let name_w = to_wide("app");
        let msg_w = to_wide("ffi_test_message");

        crate::ffi::pt_log_init(module_w.as_ptr(), base_w.as_ptr(), name_w.as_ptr());
        crate::ffi::pt_log_info(msg_w.as_ptr());
        crate::ffi::pt_log_flush();

        let content = read_logs(&base.join("ffimod"));
        assert!(
            content.contains("ffi_test_message"),
            "FFI log cycle failed: {content}"
        );
    }

    // ── 15 ───────────────────────────────────────────────────────────
    #[test]
    fn test_empty_message_no_crash() {
        let base = test_dir("empty_msg");
        let mut lg = Logger::new("m", base.to_str().unwrap(), "app").unwrap();
        lg.info("");
        lg.flush();
        let c = read_logs(lg.log_dir());
        assert!(c.contains("[INFO]"));
    }

    // ── 16 ───────────────────────────────────────────────────────────
    #[test]
    fn test_unicode_messages_preserved() {
        let base = test_dir("unicode");
        let mut lg = Logger::new("m", base.to_str().unwrap(), "app").unwrap();
        let msg = "日本語テスト 🦀 Ñoño";
        lg.info(msg);
        lg.flush();
        assert!(read_logs(lg.log_dir()).contains(msg));
    }

    // ── 17 ───────────────────────────────────────────────────────────
    #[test]
    fn test_log_multiple_messages() {
        let base = test_dir("multi_msg");
        let mut lg = Logger::new("m", base.to_str().unwrap(), "app").unwrap();
        for i in 0..10 {
            lg.info(&format!("msg_{i}"));
        }
        lg.flush();
        let c = read_logs(lg.log_dir());
        for i in 0..10 {
            assert!(c.contains(&format!("msg_{i}")), "missing msg_{i}");
        }
    }

    // ── 18 ───────────────────────────────────────────────────────────
    #[test]
    fn test_ffi_all_levels() {
        let _lock = FFI_LOCK.lock().unwrap();
        let base = test_dir("ffi_levels");

        let base_w = to_wide(base.to_str().unwrap());
        let module_w = to_wide("ffilev");
        let name_w = to_wide("app");

        crate::ffi::pt_log_init(module_w.as_ptr(), base_w.as_ptr(), name_w.as_ptr());

        crate::ffi::pt_log_trace(to_wide("ffi_TRACE_msg").as_ptr());
        crate::ffi::pt_log_debug(to_wide("ffi_DEBUG_msg").as_ptr());
        crate::ffi::pt_log_info(to_wide("ffi_INFO_msg").as_ptr());
        crate::ffi::pt_log_warn(to_wide("ffi_WARN_msg").as_ptr());
        crate::ffi::pt_log_error(to_wide("ffi_ERROR_msg").as_ptr());
        crate::ffi::pt_log_flush();

        let c = read_logs(&base.join("ffilev"));
        for tag in ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] {
            assert!(c.contains(&format!("[{tag}]")), "missing [{tag}]");
            assert!(
                c.contains(&format!("ffi_{tag}_msg")),
                "missing ffi_{tag}_msg"
            );
        }
    }
}
