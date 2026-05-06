use crate::cli::UpdateArgs;
use crate::error::QsmxtError;
use std::env;
use std::io::{self, Write};
use std::process::Command;

const REPO: &str = "astewartau/qsmxt.rs";

/// Fetches latest release info from GitHub API. Returns (tag, release_notes, html_url).
fn fetch_latest_release() -> crate::Result<(String, String, String)> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);

    let mut cmd = Command::new("curl");
    cmd.args(["-fsSL", "-H", "Accept: application/vnd.github+json"]);
    if let Ok(token) = env::var("GITHUB_TOKEN") {
        cmd.args(["-H", &format!("Authorization: token {}", token)]);
    }
    cmd.arg(&url);

    let output = cmd.output().map_err(|e| {
        QsmxtError::Update(format!("Failed to run curl: {}", e))
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(QsmxtError::Update(format!(
            "Failed to fetch release info from GitHub: {}",
            stderr.trim()
        )));
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        QsmxtError::Update(format!("Failed to parse GitHub API response: {}", e))
    })?;

    let tag = json["tag_name"]
        .as_str()
        .ok_or_else(|| QsmxtError::Update("No tag_name in release response".to_string()))?
        .to_string();

    let notes = json["body"].as_str().unwrap_or("").to_string();
    let html_url = json["html_url"].as_str().unwrap_or("").to_string();

    Ok((tag, notes, html_url))
}

/// Strips a leading 'v' from a version string if present.
fn strip_v(s: &str) -> &str {
    s.strip_prefix('v').unwrap_or(s)
}

/// Detects the install directory (directory containing the current executable).
fn install_dir() -> crate::Result<std::path::PathBuf> {
    let exe = env::current_exe().map_err(|e| {
        QsmxtError::Update(format!("Cannot determine current executable path: {}", e))
    })?;
    exe.parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| QsmxtError::Update("Cannot determine install directory".to_string()))
}

/// Detects OS/arch target triple (matching install.sh conventions).
fn detect_target() -> crate::Result<&'static str> {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    { return Ok("x86_64-unknown-linux-musl"); }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    { return Ok("aarch64-unknown-linux-gnu"); }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    { return Ok("x86_64-apple-darwin"); }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { return Ok("aarch64-apple-darwin"); }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    { return Ok("x86_64-pc-windows-msvc"); }

    #[allow(unreachable_code)]
    Err(QsmxtError::Update(format!(
        "Unsupported platform: {} {}",
        env::consts::OS,
        env::consts::ARCH,
    )))
}

/// Downloads and installs the release for the given tag.
fn install_release(tag: &str) -> crate::Result<()> {
    let target = detect_target()?;
    let dir = install_dir()?;

    #[cfg(target_os = "windows")]
    let archive_ext = "zip";
    #[cfg(not(target_os = "windows"))]
    let archive_ext = "tar.gz";

    let url = format!(
        "https://github.com/{}/releases/download/{}/qsmxt-{}-{}.{}",
        REPO, tag, tag, target, archive_ext
    );

    println!("Downloading qsmxt {}...", tag);

    // Create temp dir
    let tmp = tempfile::tempdir().map_err(|e| {
        QsmxtError::Update(format!("Failed to create temp directory: {}", e))
    })?;

    let archive_path = tmp.path().join(format!("qsmxt.{}", archive_ext));

    // Download
    let status = Command::new("curl")
        .args(["-fsSL", &url, "-o"])
        .arg(&archive_path)
        .status()
        .map_err(|e| QsmxtError::Update(format!("Failed to run curl: {}", e)))?;

    if !status.success() {
        return Err(QsmxtError::Update(format!(
            "Failed to download release from {}",
            url
        )));
    }

    // Extract
    #[cfg(not(target_os = "windows"))]
    {
        let status = Command::new("tar")
            .args(["xzf"])
            .arg(&archive_path)
            .arg("-C")
            .arg(tmp.path())
            .status()
            .map_err(|e| QsmxtError::Update(format!("Failed to extract archive: {}", e)))?;

        if !status.success() {
            return Err(QsmxtError::Update("Failed to extract archive".to_string()));
        }
    }

    #[cfg(target_os = "windows")]
    {
        // On Windows, use PowerShell to extract
        let status = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}'",
                    archive_path.display(),
                    tmp.path().display()
                ),
            ])
            .status()
            .map_err(|e| QsmxtError::Update(format!("Failed to extract archive: {}", e)))?;

        if !status.success() {
            return Err(QsmxtError::Update("Failed to extract archive".to_string()));
        }
    }

    // Determine binary name
    #[cfg(target_os = "windows")]
    let bin_name = "qsmxt.exe";
    #[cfg(not(target_os = "windows"))]
    let bin_name = "qsmxt";

    let extracted = tmp.path().join(bin_name);
    let dest = dir.join(bin_name);

    if !extracted.exists() {
        return Err(QsmxtError::Update(format!(
            "Expected binary '{}' not found in archive",
            bin_name
        )));
    }

    // Install — try direct move first, fall back to sudo on Unix
    let moved = std::fs::rename(&extracted, &dest);
    if moved.is_err() {
        // rename can fail across filesystems or due to permissions; try copy
        if std::fs::copy(&extracted, &dest).is_err() {
            #[cfg(not(target_os = "windows"))]
            {
                println!("Installing to {} (requires sudo)...", dir.display());
                let status = Command::new("sudo")
                    .args(["cp"])
                    .arg(&extracted)
                    .arg(&dest)
                    .status()
                    .map_err(|e| {
                        QsmxtError::Update(format!("Failed to install with sudo: {}", e))
                    })?;

                if !status.success() {
                    return Err(QsmxtError::Update(
                        "Failed to install binary (sudo cp failed)".to_string(),
                    ));
                }
            }
            #[cfg(target_os = "windows")]
            {
                return Err(QsmxtError::Update(format!(
                    "Failed to copy binary to {}",
                    dest.display()
                )));
            }
        }
    }

    // Ensure executable on Unix
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&dest) {
            let mut perms = meta.permissions();
            perms.set_mode(perms.mode() | 0o111);
            let _ = std::fs::set_permissions(&dest, perms);
        }
    }

    println!("Updated qsmxt to {} at {}", tag, dest.display());
    Ok(())
}

pub fn execute(args: UpdateArgs) -> crate::Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");

    println!("Current version: {}", current_version);
    println!("Checking for updates...");

    let (latest_tag, notes, html_url) = fetch_latest_release()?;
    let latest_version = strip_v(&latest_tag);

    if latest_version == strip_v(current_version) {
        println!("You are already running the latest version ({}).", current_version);
        return Ok(());
    }

    println!("New version available: {} -> {}", current_version, latest_tag);

    if !html_url.is_empty() {
        println!("Release: {}", html_url);
    }

    if !notes.is_empty() {
        println!();
        println!("Release notes:");
        println!("{}", notes);
        println!();
    }

    let should_update = if args.yes {
        true
    } else {
        print!("Do you want to update? [y/N] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
    };

    if should_update {
        install_release(&latest_tag)?;
    } else {
        println!("Update cancelled.");
    }

    Ok(())
}
