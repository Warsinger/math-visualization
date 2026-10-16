//! Keep launch failures discoverable even when Windows has no console.
use std::path::Path;

fn write_log(path: &Path, message: &str) -> std::io::Result<()> {
    use std::io::Write;
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("Missing log directory"))?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    writeln!(file, "Math Playground startup failed\n{message}")?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

pub fn report(error: &str) {
    let mut message = format!("Math Playground could not start.\n\n{error}");
    if let Some(base) = directories::BaseDirs::new() {
        let path = base
            .data_local_dir()
            .join("MathPlayground/startup-error.log");
        match write_log(&path, error) {
            Ok(()) => message.push_str(&format!("\n\nDetails saved to {}", path.display())),
            Err(e) => message.push_str(&format!("\n\nCould not save startup log: {e}")),
        }
    }
    eprintln!("{message}");
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        // Replace embedded NULs so the native dialog cannot silently truncate errors.
        let text: Vec<u16> = message
            .replace('\0', "�")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let title: Vec<u16> = "Math Playground — Startup error\0".encode_utf16().collect();
        // SAFETY: both pointers reference live, NUL-terminated UTF-16 buffers for
        // the duration of this synchronous call; there is no owner window yet.
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                text.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONERROR,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_log_creates_directory_and_atomically_replaces_previous_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app/startup-error.log");
        write_log(&path, "First error").unwrap();
        write_log(&path, "Second error\nrenderer unavailable").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("Second error\nrenderer unavailable"));
        assert!(!text.contains("First error"));
        assert_eq!(
            std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
            1
        );
        assert!(write_log(dir.path(), "Cannot replace a directory").is_err());
    }
}
