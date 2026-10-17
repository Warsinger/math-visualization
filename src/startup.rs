//! Keep launch failures discoverable even when Windows has no console.
pub fn report(error: &str, log: &crate::logging::Log) {
    tracing::error!(target: "math_playground", event = "startup_failed", error = %crate::truncate(error, 4096));
    let message = format!(
        "Math Playground could not start.\n\n{error}\n\n{}",
        log.description()
    );
    eprintln!("{message}");
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
        let text: Vec<u16> = message
            .replace('\0', "�")
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let title: Vec<u16> = "Math Playground — Startup error\0".encode_utf16().collect();
        // SAFETY: buffers are live, NUL-terminated UTF-16 throughout this synchronous call.
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
