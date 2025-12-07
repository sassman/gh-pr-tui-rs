//! Browser utilities
//!
//! Functions for opening URLs in the system's default browser.

/// Open a URL in the system's default browser
///
/// Uses platform-specific commands:
/// - macOS: `open`
/// - Linux: `xdg-open`
/// - Windows: `cmd /C start`
pub async fn open_url(url: String) {
    #[cfg(target_os = "macos")]
    let result = tokio::process::Command::new("open").arg(&url).spawn();

    #[cfg(target_os = "linux")]
    let result = tokio::process::Command::new("xdg-open").arg(&url).spawn();

    #[cfg(target_os = "windows")]
    let result = tokio::process::Command::new("cmd")
        .args(["/C", "start", &url])
        .spawn();

    if let Err(e) = result {
        log::error!("Failed to open URL in browser: {}", e);
    }
}
