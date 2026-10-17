//! Synchronous, bounded JSON Lines tracing for native builds.
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tracing_subscriber::{
    filter::LevelFilter, fmt::MakeWriter, layer::SubscriberExt, util::SubscriberInitExt,
};
const LIMIT: u64 = 2 * 1024 * 1024;
const RECORD_LIMIT: usize = 64 * 1024;

pub struct GeometryContext {
    pub request: String,
    request_truncated: bool,
    mode: math_playground::scene::PlotMode,
    surface: math_playground::geometry::Sampling,
    curve: math_playground::curve::CurveSampling,
}
impl GeometryContext {
    pub fn new(
        request: &str,
        mode: math_playground::scene::PlotMode,
        surface: math_playground::geometry::Sampling,
        curve: math_playground::curve::CurveSampling,
    ) -> Self {
        Self {
            request: crate::truncate(request, 1024),
            request_truncated: request.len() > 1024,
            mode,
            surface,
            curve,
        }
    }
    pub fn rejected(&self, error: &str) {
        tracing::warn!(target:"math_playground", event="geometry_rejected", request=%self.request, request_truncated=self.request_truncated, error=%crate::truncate(error,1024), error_truncated=error.len()>1024, mode=?self.mode, surface_extent=?self.surface.extent, surface_resolution=self.surface.resolution, curve_extent=?self.curve.extent, curve_samples=self.curve.samples);
    }
    pub fn disconnected(&self) {
        tracing::error!(target:"math_playground", event="geometry_worker_disconnected", request=%self.request, request_truncated=self.request_truncated, error="Geometry worker stopped before returning a result", mode=?self.mode, surface_extent=?self.surface.extent, surface_resolution=self.surface.resolution, curve_extent=?self.curve.extent, curve_samples=self.curve.samples);
    }
}

fn write_diagnostic(writer: &mut impl Write, message: &str) {
    // Unlike eprintln!, a failed stderr write must never panic, especially
    // when this fallback is reached from Record::drop or the panic hook.
    let _ = writeln!(writer, "{message}");
}
pub fn diagnostic(message: &str) {
    write_diagnostic(&mut io::stderr().lock(), message);
}

#[derive(Clone)]
pub struct Log {
    path: Option<PathBuf>,
    failure: Arc<Mutex<Option<String>>>,
}
impl Log {
    fn new(path: Option<PathBuf>) -> Self {
        Self {
            path,
            failure: Default::default(),
        }
    }
    fn fail(&self, error: impl ToString) {
        if let Ok(mut failure) = self.failure.lock() {
            *failure = Some(error.to_string());
        }
        diagnostic(&format!(
            "Application logging unavailable: {}",
            error.to_string()
        ));
    }
    pub fn description(&self) -> String {
        if let Ok(failure) = self.failure.lock()
            && let Some(error) = failure.as_ref()
        {
            return format!("Application log unavailable: {error}");
        }
        self.path.as_ref().map_or_else(
            || "Application log location unavailable.".into(),
            |p| format!("Application log: {}", p.display()),
        )
    }
    fn append(&self, bytes: &[u8], limit: u64) -> io::Result<()> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| io::Error::other("No local data directory"))?;
        append(path, bytes, limit)
    }
}
fn append(path: &Path, bytes: &[u8], limit: u64) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Missing log directory"))?;
    std::fs::create_dir_all(parent)?;
    // Stable lock file coordinates rotation across simultaneous app instances.
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path.with_extension("lock"))?;
    lock.try_lock().map_err(io::Error::other)?;
    let size = path.metadata().map(|m| m.len()).unwrap_or(0);
    if size > 0 && size + bytes.len() as u64 > limit {
        let oldest = path.with_extension("jsonl.2");
        match std::fs::remove_file(&oldest) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
        let previous = path.with_extension("jsonl.1");
        if previous.exists() {
            std::fs::rename(&previous, &oldest)?;
        }
        std::fs::rename(path, previous)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.flush()?;
    file.sync_data()
}
pub struct Record {
    log: Log,
    bytes: Vec<u8>,
    oversized: bool,
}
impl Write for Record {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.bytes.len().saturating_add(bytes.len()) <= RECORD_LIMIT && !self.oversized {
            self.bytes.extend_from_slice(bytes);
        } else {
            self.oversized = true;
            self.bytes.clear();
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Drop for Record {
    fn drop(&mut self) {
        if self.oversized {
            self.log.fail("Log event exceeded 64 KiB; record omitted");
        } else if !self.bytes.is_empty()
            && let Err(e) = self.log.append(&self.bytes, LIMIT)
        {
            self.log.fail(e);
        }
    }
}
impl<'a> MakeWriter<'a> for Log {
    type Writer = Record;
    fn make_writer(&'a self) -> Self::Writer {
        Record {
            log: self.clone(),
            bytes: Vec::new(),
            oversized: false,
        }
    }
}
pub fn init() -> Log {
    let log = Log::new(directories::BaseDirs::new().map(|b| {
        b.data_local_dir()
            .join("MathPlayground/logs/application.jsonl")
    }));
    let level = match std::env::var("MATH_PLAYGROUND_LOG").as_deref() {
        Ok("trace") => LevelFilter::TRACE,
        Ok("debug") => LevelFilter::DEBUG,
        Ok("warn") => LevelFilter::WARN,
        Ok("error") => LevelFilter::ERROR,
        Ok("off") => LevelFilter::OFF,
        _ => LevelFilter::INFO,
    };
    let filter = tracing_subscriber::filter::Targets::new()
        .with_default(LevelFilter::WARN)
        .with_target("math_playground", level);
    let layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(log.clone());
    if let Err(e) = tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .try_init()
    {
        log.fail(e);
    }
    if let Err(e) = log.append(&[], LIMIT) {
        log.fail(e);
    }
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Do not log arbitrary panic payloads which might include request contents.
        tracing::error!(target: "math_playground", event = "panic", location = %info.location().map(|l| l.to_string()).unwrap_or_default());
        previous(info);
    }));
    tracing::info!(target: "math_playground", event = "application_start", version = env!("CARGO_PKG_VERSION"), pid = std::process::id());
    log
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_records_include_bounded_request_error_and_sampling_context() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("application.jsonl");
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_ansi(false)
            .with_writer(Log::new(Some(path.clone())))
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            let context = GeometryContext::new(
                "plot 2πx + \"bad\"\n",
                math_playground::scene::PlotMode::Auto,
                Default::default(),
                Default::default(),
            );
            context.rejected("Unknown name 'bad'");
            context.disconnected();
            let context = GeometryContext::new(
                &"π".repeat(2000),
                math_playground::scene::PlotMode::ThreeD,
                Default::default(),
                Default::default(),
            );
            context.rejected(&"x".repeat(2000));
        });
        let text = std::fs::read_to_string(path).unwrap();
        let rows: Vec<serde_json::Value> = text
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["fields"]["request"], "plot 2πx + \"bad\"\n");
        assert_eq!(rows[0]["fields"]["error"], "Unknown name 'bad'");
        assert_eq!(rows[0]["fields"]["surface_resolution"], 24);
        assert_eq!(rows[1]["fields"]["event"], "geometry_worker_disconnected");
        assert_eq!(rows[2]["fields"]["request_truncated"], true);
        assert_eq!(rows[2]["fields"]["error_truncated"], true);
        assert!(
            rows[2]["fields"]["request"]
                .as_str()
                .unwrap()
                .chars()
                .count()
                <= 1025
        );
        assert!(rows[2]["fields"]["error"].as_str().unwrap().chars().count() <= 1025);
    }
    #[test]
    fn failed_fallback_writer_does_not_panic() {
        struct Broken;
        impl Write for Broken {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
        }
        write_diagnostic(&mut Broken, "Application logging unavailable");
        write_diagnostic(
            &mut Broken,
            "Original startup failure must still reach its dialog",
        );
    }
    #[test]
    fn contended_log_lock_is_nonfatal_and_later_events_recover() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("application.jsonl");
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(path.with_extension("lock"))
            .unwrap();
        lock.try_lock().unwrap();
        let log = Log::new(Some(path.clone()));
        let mut record = log.make_writer();
        record.write_all(b"blocked\n").unwrap();
        drop(record);
        assert!(log.description().contains("unavailable"));
        assert!(!path.exists());
        drop(lock);
        let mut record = log.make_writer();
        record.write_all(b"recovered\n").unwrap();
        drop(record);
        assert_eq!(std::fs::read_to_string(path).unwrap(), "recovered\n");
    }
    #[test]
    fn structured_records_append_and_preserve_utc_timestamps() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("logs/application.jsonl");
        let log = Log::new(Some(path.clone()));
        let subscriber = tracing_subscriber::fmt()
            .json()
            .with_ansi(false)
            .with_writer(log)
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(event = "first", count = 3);
            tracing::error!(event = "second", reason = "failure\nwith newline");
        });
        let text = std::fs::read_to_string(path).unwrap();
        let rows: Vec<serde_json::Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["fields"]["count"], 3);
        assert_eq!(rows[1]["level"], "ERROR");
        for row in rows {
            let timestamp = row["timestamp"].as_str().unwrap();
            assert!(timestamp.ends_with('Z'));
            assert!(chrono::DateTime::parse_from_rfc3339(timestamp).is_ok());
            assert!(row["target"].is_string());
        }
    }
    #[test]
    fn rotation_retains_two_archives_and_io_failure_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("application.jsonl");
        for i in 0..6 {
            append(&path, format!("{i}\n").as_bytes(), 3).unwrap();
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "5\n");
        assert_eq!(
            std::fs::read_to_string(path.with_extension("jsonl.1")).unwrap(),
            "4\n"
        );
        assert_eq!(
            std::fs::read_to_string(path.with_extension("jsonl.2")).unwrap(),
            "3\n"
        );
        let log = Log::new(Some(dir.path().to_owned()));
        let mut record = log.make_writer();
        record.write_all(b"event\n").unwrap();
        drop(record);
        assert!(log.description().contains("unavailable"));
        let log = Log::new(Some(path.clone()));
        let mut record = log.make_writer();
        record.write_all(&vec![0; RECORD_LIMIT + 1]).unwrap();
        drop(record);
        assert!(log.description().contains("64 KiB"));
        assert_eq!(std::fs::read_to_string(path).unwrap(), "5\n");
    }
}
