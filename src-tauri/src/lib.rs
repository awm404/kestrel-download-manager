use std::net::TcpStream;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use ureq::ResponseExt;

use tauri::{
    image::Image,
    menu::{ContextMenu, IconMenuItem, Menu, PredefinedMenuItem},
    tray::{MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, WindowEvent,
};

struct AppState {
    minimize_to_tray: Arc<AtomicBool>,
}

const EXTENSION_DIR_NAME: &str = "kestrel-extension";

fn user_home() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Directory holding `aria2.conf`, the aria2 session file and Kestrel's log.
/// Override with the `KESTREL_ARIA2_HOME` environment variable.
pub fn aria2_home() -> PathBuf {
    std::env::var_os("KESTREL_ARIA2_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| user_home().join(".aria2"))
}

fn aria2_conf_path() -> PathBuf {
    aria2_home().join("aria2.conf")
}

fn default_download_dir() -> PathBuf {
    user_home().join("Downloads")
}

pub fn log_line(scope: &str, msg: &str) {
    use std::io::Write;
    let home = aria2_home();
    let _ = std::fs::create_dir_all(&home);
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join("tauri_app.log"))
    {
        let _ = writeln!(
            f,
            "[{:?}] [{}] {}",
            std::time::SystemTime::now(),
            scope,
            msg
        );
    }
}

fn log_app(msg: &str) {
    log_line("app", msg);
}

/// Clears the Windows read-only attribute so the file can be renamed or deleted.
/// Not done elsewhere: on Unix `set_readonly(false)` would make the file world-writable.
#[allow(clippy::permissions_set_readonly_false)]
fn clear_readonly(path: &Path) {
    if !cfg!(windows) {
        return;
    }
    if let Ok(metadata) = std::fs::metadata(path) {
        let mut perms = metadata.permissions();
        if perms.readonly() {
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(path, perms);
        }
    }
}

/// Reads a non-empty `key=value` entry from aria2.conf.
fn conf_value(key: &str) -> Option<String> {
    let content = std::fs::read_to_string(aria2_conf_path()).ok()?;
    let prefix = format!("{}=", key);
    content
        .lines()
        .filter_map(|line| line.trim_start().strip_prefix(prefix.as_str()))
        .map(|value| value.trim().to_string())
        .find(|value| !value.is_empty())
}

/// Reads the `dir=` entry from aria2.conf, falling back to the user's Downloads folder.
fn configured_download_dir() -> PathBuf {
    conf_value("dir")
        .map(PathBuf::from)
        .unwrap_or_else(default_download_dir)
}

const RPC_SECRET_FILE: &str = "kestrel-rpc-secret";

/// Secret for aria2's JSON-RPC interface. Without one, any web page can drive aria2 through
/// `http://127.0.0.1:6800/jsonrpc` (aria2 also accepts RPC over plain GET requests), e.g. to
/// download files into arbitrary folders. Uses `rpc-secret` from aria2.conf when present,
/// otherwise a random secret generated once and stored next to it.
fn rpc_secret() -> String {
    if let Some(secret) = conf_value("rpc-secret") {
        return secret;
    }
    let path = aria2_home().join(RPC_SECRET_FILE);
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let existing = existing.trim();
        if existing.len() >= 16 {
            return existing.to_string();
        }
    }
    let mut bytes = [0u8; 24];
    if let Err(e) = getrandom::getrandom(&mut bytes) {
        log_app(&format!("Could not generate an RPC secret: {:?}", e));
        return String::new();
    }
    let secret: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    let _ = std::fs::create_dir_all(aria2_home());
    if let Err(e) = std::fs::write(&path, &secret) {
        log_app(&format!(
            "Could not save the RPC secret to {:?}: {:?}",
            path, e
        ));
    }
    secret
}

/// Loopback address the browser extension reads to learn the RPC secret.
/// There is no `Access-Control-Allow-Origin` header, so a web page's `fetch` cannot
/// read the response. The extension can, because its host permission is not subject
/// to CORS. Anything running as this Windows user can already read the secret file.
const EXTENSION_SECRET_ADDR: &str = "127.0.0.1:6801";

fn start_extension_secret_server() {
    std::thread::spawn(|| {
        let listener = match std::net::TcpListener::bind(EXTENSION_SECRET_ADDR) {
            Ok(listener) => listener,
            Err(err) => {
                log_app(&format!(
                    "Extension secret port {EXTENSION_SECRET_ADDR} is unavailable: {err}"
                ));
                return;
            }
        };
        log_app("Extension can read the RPC secret from the local pairing port");
        for conn in listener.incoming() {
            let Ok(mut stream) = conn else { continue };
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
            let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(2)));
            let mut buf = [0u8; 1024];
            let n = match std::io::Read::read(&mut stream, &mut buf) {
                Ok(n) => n,
                Err(_) => continue,
            };
            let req = String::from_utf8_lossy(&buf[..n]);
            let secret = rpc_secret();
            let allowed = req.starts_with("GET /kestrel-rpc-secret ") && !secret.is_empty();
            let (status, body) = if allowed {
                ("200 OK", secret)
            } else {
                ("404 Not Found", String::new())
            };
            let resp = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{body}",
                body.len()
            );
            let _ = std::io::Write::write_all(&mut stream, resp.as_bytes());
        }
    });
}

/// Locates the download engine. Search order:
/// `KESTREL_ARIA2C` env var, Kestrel Engine or aria2c next to the Kestrel executable,
/// the WinGet links folder, any WinGet-installed aria2 package, then `aria2c` on `PATH`.
fn find_aria2c() -> PathBuf {
    let (engine_name, exe_name) = if cfg!(windows) {
        ("kestrel-engine.exe", "aria2c.exe")
    } else {
        ("kestrel-engine", "aria2c")
    };

    if let Some(p) = std::env::var_os("KESTREL_ARIA2C").map(PathBuf::from) {
        if p.is_file() {
            return p;
        }
    }

    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        for name in [engine_name, exe_name] {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return candidate;
            }
        }
    }

    #[cfg(windows)]
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        let winget = local.join("Microsoft").join("WinGet");
        let link = winget.join("Links").join(exe_name);
        if link.is_file() {
            return link;
        }
        if let Ok(packages) = std::fs::read_dir(winget.join("Packages")) {
            for pkg in packages.flatten() {
                if !pkg.file_name().to_string_lossy().starts_with("aria2.aria2") {
                    continue;
                }
                if let Ok(builds) = std::fs::read_dir(pkg.path()) {
                    for build in builds.flatten() {
                        let candidate = build.path().join(exe_name);
                        if candidate.is_file() {
                            return candidate;
                        }
                    }
                }
            }
        }
    }

    PathBuf::from(exe_name)
}

/// Returns candidate locations of the companion browser extension, most specific first.
fn extension_dir_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(res) = app.path().resource_dir() {
        candidates.push(res.join(EXTENSION_DIR_NAME));
    }
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors().skip(1).take(5) {
            candidates.push(ancestor.join(EXTENSION_DIR_NAME));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(EXTENSION_DIR_NAME));
    }
    candidates.push(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(EXTENSION_DIR_NAME),
    );
    candidates
}

/// Opens `path` itself when it is a directory, or the directory that contains it
/// when it is a file. `explorer /select` is not used: with an Explorer window
/// already open it often stays where it was instead of navigating.
#[tauri::command]
fn open_in_explorer(path: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        let normalized = path.replace('/', "\\");
        let raw = PathBuf::from(&normalized);
        let folder = if raw.is_dir() {
            raw
        } else if raw.is_file() {
            raw.parent()
                .filter(|parent| parent.is_dir())
                .map(Path::to_path_buf)
                .ok_or_else(|| "Could not find the folder for this file".to_string())?
        } else {
            return Err("That folder is not on disk yet".to_string());
        };

        let verb: Vec<u16> = std::ffi::OsStr::new("explore")
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let target: Vec<u16> = folder
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let code = unsafe {
            windows_sys::Win32::UI::Shell::ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
            ) as isize
        };
        if code <= 32 {
            return Err(format!("Explorer could not open {}", folder.display()));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("Opening a folder is only supported on Windows".to_string())
    }
}

#[tauri::command]
fn open_extension_dir(app: AppHandle) -> Result<String, String> {
    log_app("open_extension_dir invoked");
    let found = extension_dir_candidates(&app)
        .into_iter()
        .find(|c| c.is_dir())
        .ok_or_else(|| {
            format!(
                "Could not find the '{}' folder next to Kestrel",
                EXTENSION_DIR_NAME
            )
        })?;
    let found = std::fs::canonicalize(&found).unwrap_or(found);
    let mut s = found.to_string_lossy().to_string();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        s = stripped.to_string();
    }
    open_in_explorer(s.clone())?;
    Ok(s)
}

#[tauri::command]
fn open_downloaded_file(path: String) -> Result<bool, String> {
    log_app(&format!("open_downloaded_file invoked for: {}", path));
    let normalized = path.replace('/', "\\");
    let p = std::path::Path::new(&normalized);
    if p.exists() {
        log_app(&format!("Target path exists: {}", normalized));
        match open::that_detached(&normalized) {
            Ok(_) => {
                log_app("open::that_detached succeeded");
                Ok(true)
            }
            Err(e) => {
                log_app(&format!(
                    "open::that_detached error: {:?}, falling back to explorer",
                    e
                ));
                let _ = Command::new("explorer").arg(&normalized).spawn();
                Ok(true)
            }
        }
    } else {
        log_app(&format!("Target path NOT found on disk: {}", normalized));
        Ok(false)
    }
}

#[tauri::command]
fn open_browser_url(url: String) -> Result<bool, String> {
    log_app(&format!("open_browser_url invoked for: {}", url));
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("URL cannot be empty".to_string());
    }
    match open::that_detached(trimmed) {
        Ok(_) => {
            log_app("open_browser_url succeeded");
            Ok(true)
        }
        Err(e) => {
            log_app(&format!(
                "open_browser_url error: {:?}, falling back to explorer",
                e
            ));
            let _ = Command::new("explorer").arg(trimmed).spawn();
            Ok(true)
        }
    }
}

#[tauri::command]
fn check_file_exists(path: String) -> bool {
    let normalized = path.replace('/', "\\");
    std::path::Path::new(&normalized).exists()
}

#[tauri::command]
fn delete_download_file(path: String) -> Result<bool, String> {
    log_app(&format!("delete_download_file invoked for: {}", path));

    // Normalize path for Windows: replace forward slashes and collapse duplicate separators
    let mut normalized = path.replace('/', "\\");
    while normalized.contains("\\\\") && !normalized.starts_with("\\\\") {
        normalized = normalized.replace("\\\\", "\\");
    }
    log_app(&format!(
        "delete_download_file normalized path: {}",
        normalized
    ));

    let p = std::path::PathBuf::from(&normalized);
    let mut deleted = false;

    if p.exists() {
        // Attempt file or directory removal with retry in case aria2c / AV still holds a transient handle
        for attempt in 1..=5 {
            if p.is_file() {
                clear_readonly(&p);
                match std::fs::remove_file(&p) {
                    Ok(_) => {
                        deleted = true;
                        log_app(&format!(
                            "delete_download_file: successfully removed file {:?}",
                            p
                        ));
                        break;
                    }
                    Err(e) => {
                        log_app(&format!(
                            "delete_download_file: attempt {} remove_file error: {:?}",
                            attempt, e
                        ));
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            } else if p.is_dir() {
                match std::fs::remove_dir_all(&p) {
                    Ok(_) => {
                        deleted = true;
                        log_app(&format!(
                            "delete_download_file: successfully removed dir {:?}",
                            p
                        ));
                        break;
                    }
                    Err(e) => {
                        log_app(&format!(
                            "delete_download_file: attempt {} remove_dir_all error: {:?}",
                            attempt, e
                        ));
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        }
    } else {
        log_app(&format!(
            "delete_download_file: target path does not exist on disk: {:?}",
            p
        ));
    }

    // Also remove companion .aria2 control file if present
    let aria2_control = format!("{}.aria2", normalized);
    let p_ctrl = std::path::Path::new(&aria2_control);
    if p_ctrl.exists() {
        clear_readonly(p_ctrl);
        let _ = std::fs::remove_file(p_ctrl);
        log_app(&format!(
            "delete_download_file: removed companion aria2 control file {:?}",
            p_ctrl
        ));
    }

    Ok(deleted)
}

#[tauri::command]
fn rename_download_file(old_path: String, new_name: String) -> Result<String, String> {
    log_app(&format!(
        "rename_download_file invoked: old_path='{}', new_name='{}'",
        old_path, new_name
    ));

    let clean_new_name = new_name.trim();
    if clean_new_name.is_empty() {
        return Err("New filename cannot be empty".to_string());
    }

    // Disallow illegal characters in Windows filenames
    if clean_new_name
        .chars()
        .any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|'))
    {
        return Err("Filename contains illegal characters (\\ / : * ? \" < > |)".to_string());
    }

    let mut normalized_old = old_path.replace('/', "\\");
    while normalized_old.contains("\\\\") && !normalized_old.starts_with("\\\\") {
        normalized_old = normalized_old.replace("\\\\", "\\");
    }

    let old_p = std::path::PathBuf::from(&normalized_old);
    if !old_p.exists() {
        log_app(&format!(
            "rename_download_file: source file not found: {:?}",
            old_p
        ));
        return Err("Source file does not exist on disk".to_string());
    }

    let parent = match old_p.parent() {
        Some(p) => p,
        None => return Err("Cannot determine parent directory".to_string()),
    };

    let new_p = parent.join(clean_new_name);

    if new_p == old_p {
        return Ok(normalized_old);
    }

    if new_p.exists() {
        return Err(format!(
            "A file named '{}' already exists in the same folder",
            clean_new_name
        ));
    }

    clear_readonly(&old_p);

    // Rename main file
    if let Err(e) = std::fs::rename(&old_p, &new_p) {
        log_app(&format!(
            "rename_download_file: failed to rename {:?} -> {:?}: {:?}",
            old_p, new_p, e
        ));
        return Err(format!("Failed to rename file: {}", e));
    }

    // Rename companion .aria2 file if present
    let old_aria2 = format!("{}.aria2", normalized_old);
    let new_aria2 = format!("{}.aria2", new_p.to_string_lossy());
    let old_aria2_p = std::path::Path::new(&old_aria2);
    if old_aria2_p.exists() {
        let _ = std::fs::rename(old_aria2_p, &new_aria2);
        log_app(&format!(
            "rename_download_file: companion .aria2 file renamed to {}",
            new_aria2
        ));
    }

    let new_path_str = new_p.to_string_lossy().to_string();
    log_app(&format!(
        "rename_download_file: successfully renamed to {}",
        new_path_str
    ));
    Ok(new_path_str)
}

#[tauri::command]
fn resolve_web_download_url(url: String) -> Result<serde_json::Value, String> {
    log_app(&format!("resolve_web_download_url requested for: {}", url));
    let trimmed = url.trim();

    // Check if it is a TechPowerUp download page
    if trimmed.contains("techpowerup.com/download/") && !trimmed.contains("/files/") {
        let ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

        // Step 1: GET landing page
        let mut resp1 = ureq::get(trimmed)
            .header("User-Agent", ua)
            .header("Referer", trimmed)
            .call()
            .map_err(|e| format!("TechPowerUp page request failed: {}", e))?;
        let page_html = resp1
            .body_mut()
            .read_to_string()
            .map_err(|e| format!("Failed to read TechPowerUp page: {}", e))?;

        // Extract id (e.g. name="id" value="2835")
        let mut file_id = None;
        for line in page_html.lines() {
            if line.contains("name=\"id\"") || line.contains("name='id'") {
                if let Some(pos) = line.find("value=") {
                    let rest = &line[pos + 6..];
                    if let Some(quote) = rest.chars().next() {
                        if quote == '"' || quote == '\'' {
                            let val = &rest[1..];
                            if let Some(end) = val.find(quote) {
                                file_id = Some(val[..end].to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }

        let id_val =
            file_id.ok_or_else(|| "Could not locate file ID on TechPowerUp page".to_string())?;
        log_app(&format!(
            "resolve_web_download_url extracted file ID: {}",
            id_val
        ));

        // Step 2: POST id to get available server mirrors
        let step1_body = format!("id={}", id_val);
        let mut resp2 = ureq::post(trimmed)
            .header("User-Agent", ua)
            .header("Referer", trimmed)
            .header("Origin", "https://www.techpowerup.com")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send(step1_body.as_bytes())
            .map_err(|e| format!("Mirror list request failed: {}", e))?;
        let mirror_html = resp2
            .body_mut()
            .read_to_string()
            .map_err(|e| format!("Failed to read mirror list: {}", e))?;

        // Find server_ids
        let mut server_id = None;
        for line in mirror_html.lines() {
            if line.contains("name=\"server_id\"") || line.contains("name='server_id'") {
                if let Some(pos) = line.find("value=") {
                    let rest = &line[pos + 6..];
                    if let Some(quote) = rest.chars().next() {
                        if quote == '"' || quote == '\'' {
                            let val = &rest[1..];
                            if let Some(end) = val.find(quote) {
                                server_id = Some(val[..end].to_string());
                                break;
                            }
                        }
                    }
                }
            }
        }

        let chosen_server = server_id.unwrap_or_else(|| "15".to_string());
        log_app(&format!(
            "resolve_web_download_url chosen server_id: {}",
            chosen_server
        ));

        // Step 3: POST id + server_id to get direct redirect
        let step2_body = format!("id={}&server_id={}", id_val, chosen_server);
        let resp3 = ureq::post(trimmed)
            .header("User-Agent", ua)
            .header("Referer", trimmed)
            .header("Origin", "https://www.techpowerup.com")
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send(step2_body.as_bytes())
            .map_err(|e| format!("Direct mirror request failed: {}", e))?;

        let direct_url = resp3.get_uri().to_string();
        log_app(&format!(
            "resolve_web_download_url direct URL: {}",
            direct_url
        ));

        // Extract filename from direct_url if possible
        let clean_u = direct_url.split('?').next().unwrap_or("");
        let detected_filename = clean_u.split('/').next_back().map(|s: &str| s.to_string());

        return Ok(serde_json::json!({
            "resolved": true,
            "url": direct_url,
            "filename": detected_filename,
            "referer": trimmed,
            "service": "TechPowerUp"
        }));
    }

    Ok(serde_json::json!({
        "resolved": false,
        "url": trimmed
    }))
}

#[tauri::command]
fn check_is_html(path: String) -> bool {
    let mut normalized = path.replace('/', "\\");
    while normalized.contains("\\\\") && !normalized.starts_with("\\\\") {
        normalized = normalized.replace("\\\\", "\\");
    }
    let p = std::path::Path::new(&normalized);
    if let Ok(mut f) = std::fs::File::open(p) {
        use std::io::Read;
        let mut buffer = [0u8; 512];
        if let Ok(bytes_read) = f.read(&mut buffer) {
            let sample = String::from_utf8_lossy(&buffer[..bytes_read]).to_lowercase();
            if sample.contains("<html")
                || sample.contains("<!doctype html")
                || sample.contains("<head")
            {
                return true;
            }
        }
    }
    false
}

#[tauri::command]
fn save_default_dir(dir: String) -> Result<(), String> {
    set_conf_values(&[("dir", dir.replace('\\', "/"))]);
    Ok(())
}

/// Upper bound accepted by Kestrel Engine; stock aria2 rejects values above 16.
const MAX_CONNECTIONS_PER_SERVER: u32 = 64;

#[tauri::command]
fn save_connections_per_server(count: u32) -> Result<(), String> {
    if !(1..=MAX_CONNECTIONS_PER_SERVER).contains(&count) {
        return Err(format!(
            "Connections per server must be between 1 and {}",
            MAX_CONNECTIONS_PER_SERVER
        ));
    }
    set_conf_values(&[
        ("max-connection-per-server", count.to_string()),
        ("split", count.to_string()),
    ]);
    Ok(())
}

/// Replaces or appends `key=value` entries in aria2.conf. Does nothing when the file is missing.
fn set_conf_values(values: &[(&str, String)]) {
    let conf_path = aria2_conf_path();
    let Ok(content) = std::fs::read_to_string(&conf_path) else {
        return;
    };
    let mut found = vec![false; values.len()];
    let mut new_lines = Vec::new();
    for line in content.lines() {
        let key = line.trim_start().split('=').next().unwrap_or("").trim();
        if let Some(i) = values.iter().position(|(k, _)| *k == key) {
            new_lines.push(format!("{}={}", values[i].0, values[i].1));
            found[i] = true;
        } else {
            new_lines.push(line.to_string());
        }
    }
    for ((k, v), _) in values.iter().zip(&found).filter(|(_, f)| !**f) {
        new_lines.push(format!("{}={}", k, v));
    }
    let _ = std::fs::write(&conf_path, new_lines.join("\n"));
}

#[tauri::command]
fn get_rpc_secret() -> String {
    rpc_secret()
}

#[tauri::command]
fn set_minimize_to_tray(state: tauri::State<AppState>, enabled: bool) {
    state.minimize_to_tray.store(enabled, Ordering::SeqCst);
}

#[tauri::command]
fn get_minimize_to_tray(state: tauri::State<AppState>) -> bool {
    state.minimize_to_tray.load(Ordering::SeqCst)
}

#[tauri::command]
fn get_clipboard_text() -> Result<String, String> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::DataExchange::{
            CloseClipboard, GetClipboardData, OpenClipboard,
        };
        use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};

        const CF_UNICODETEXT: u32 = 13;

        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return Ok(String::new());
            }

            let handle = GetClipboardData(CF_UNICODETEXT);
            if handle.is_null() {
                CloseClipboard();
                return Ok(String::new());
            }

            let ptr = GlobalLock(handle) as *const u16;
            if ptr.is_null() {
                CloseClipboard();
                return Ok(String::new());
            }

            let mut len = 0;
            while *ptr.add(len) != 0 {
                len += 1;
            }

            let slice = std::slice::from_raw_parts(ptr, len);
            let text = String::from_utf16_lossy(slice);

            GlobalUnlock(handle);
            CloseClipboard();

            Ok(text)
        }
    }
    #[cfg(not(windows))]
    {
        Ok(String::new())
    }
}

#[tauri::command]
fn notify_download_complete(title: String, message: String, file_path: Option<String>) {
    log_app(&format!(
        "notify_download_complete: {} - {:?}",
        title, file_path
    ));
    #[cfg(windows)]
    {
        std::thread::spawn(move || {
            // Values land inside an XML document that is itself a single-quoted PowerShell string.
            let escape = |s: &str| {
                s.replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;")
                    .replace('"', "&quot;")
                    .replace('\'', "''")
            };
            let escaped_title = escape(&title);
            let escaped_msg = escape(&message);

            let ps_script = format!(
                r#"
[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime] | Out-Null
$template = '<toast><visual><binding template="ToastGeneric"><text>{0}</text><text>{1}</text></binding></visual><audio src="ms-winsoundevent:Notification.Default"/></toast>'
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml($template)
$toast = New-Object Windows.UI.Notifications.ToastNotification $xml
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('{{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}}\WindowsPowerShell\v1.0\powershell.exe').Show($toast)
"#,
                escaped_title, escaped_msg
            );

            let mut cmd = Command::new("powershell");
            cmd.args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &ps_script,
            ]);
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
            let _ = cmd.output();
        });
    }
}

#[tauri::command]
fn update_tray_status(
    app: AppHandle,
    is_downloading: bool,
    speed_text: String,
    active_count: usize,
    progress_pct: f64,
    top_file_name: String,
    _default_dir: Option<String>,
) {
    if let Some(tray) = app.tray_by_id("main-tray") {
        let tooltip = if is_downloading {
            if !top_file_name.is_empty() {
                format!(
                    "Kestrel: {} ({:.0}%) • {}\n{} active transfer{}",
                    speed_text,
                    progress_pct,
                    top_file_name,
                    active_count,
                    if active_count == 1 { "" } else { "s" }
                )
            } else {
                format!(
                    "Kestrel: {} ({:.0}%) • {} active",
                    speed_text, progress_pct, active_count
                )
            }
        } else if progress_pct >= 100.0 && !speed_text.is_empty() {
            "Kestrel: All downloads finished".to_string()
        } else {
            "Kestrel Download Manager".to_string()
        };
        let _ = tray.set_tooltip(Some(&tooltip));
    }
}

#[tauri::command]
fn select_directory(default_path: Option<String>) -> Option<String> {
    log_app(&format!(
        "select_directory invoked with default: {:?}",
        default_path
    ));
    let mut dialog = rfd::FileDialog::new().set_title("Select Download Directory");
    if let Some(ref path) = default_path {
        let trimmed = path.trim();
        if !trimmed.is_empty() && std::path::Path::new(trimmed).exists() {
            dialog = dialog.set_directory(trimmed);
        }
    }
    let chosen = dialog
        .pick_folder()
        .map(|p| p.to_string_lossy().to_string());
    log_app(&format!("select_directory result: {:?}", chosen));
    chosen
}

#[tauri::command]
fn sync_window_mode(window: tauri::WebviewWindow, mode: String) {
    if let Ok(false) = window.is_maximized() {
        let (w, h) = if mode == "advanced" {
            (1200.0, 760.0)
        } else {
            (840.0, 570.0)
        };
        let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
            width: w,
            height: h,
        }));
        let _ = window.center();
    }
}

#[tauri::command]
fn window_minimize(window: tauri::WebviewWindow) {
    let _ = window.minimize();
}

#[tauri::command]
fn window_toggle_maximize(window: tauri::WebviewWindow, mode: Option<String>) {
    if let Ok(maximized) = window.is_maximized() {
        if maximized {
            let _ = window.unmaximize();
            let is_adv = mode.as_deref() == Some("advanced");
            let (w, h) = if is_adv {
                (1200.0, 760.0)
            } else {
                (840.0, 570.0)
            };
            let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize {
                width: w,
                height: h,
            }));
            let _ = window.center();
        } else {
            let _ = window.maximize();
        }
    }
}

#[tauri::command]
fn window_close(window: tauri::WebviewWindow) {
    let _ = window.close();
}

/// Opaque window colour, matching `--bg-base` of the default theme.
const OPAQUE_BACKGROUND: tauri::window::Color = tauri::window::Color(10, 13, 20, 255);

/// Real Windows build number; `GetVersionEx` is manifest-dependent and may under-report.
#[cfg(windows)]
fn windows_build_number() -> u32 {
    use windows_sys::Wdk::System::SystemServices::RtlGetVersion;
    use windows_sys::Win32::System::SystemInformation::OSVERSIONINFOW;

    let mut info: OSVERSIONINFOW = unsafe { std::mem::zeroed() };
    info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;
    if unsafe { RtlGetVersion(&mut info) } == 0 {
        info.dwBuildNumber
    } else {
        0
    }
}

/// Turns the native translucent backdrop on or off. Returns the material in use
/// ("acrylic", "mica", "blur") or `None` when the platform has no supported material.
#[tauri::command]
fn set_translucent(window: tauri::WebviewWindow, enabled: bool) -> Result<Option<String>, String> {
    if !enabled {
        window.set_effects(None).map_err(|e| e.to_string())?;
        window
            .set_background_color(Some(OPAQUE_BACKGROUND))
            .map_err(|e| e.to_string())?;
        return Ok(None);
    }

    #[cfg(windows)]
    {
        use tauri::window::{Color, Effect, EffectsBuilder};

        let build = windows_build_number();
        // Acrylic shows the windows behind; before build 22523 it is the legacy implementation,
        // which lags while dragging, so older Windows 11 builds use Mica (wallpaper tint only).
        let (effect, material) = if build >= 22523 {
            (Effect::Acrylic, "acrylic")
        } else if build >= 22000 {
            (Effect::MicaDark, "mica")
        } else if build >= 10240 {
            (Effect::Blur, "blur")
        } else {
            return Ok(None);
        };
        window
            .set_background_color(Some(Color(0, 0, 0, 0)))
            .map_err(|e| e.to_string())?;
        window
            .set_effects(
                EffectsBuilder::new()
                    .effect(effect)
                    .color(Color(10, 13, 20, 184))
                    .build(),
            )
            .map_err(|e| e.to_string())?;
        log_app(&format!("set_translucent: {} (build {})", material, build));
        Ok(Some(material.to_string()))
    }

    #[cfg(not(windows))]
    Ok(None)
}

fn is_aria2_running() -> bool {
    TcpStream::connect("127.0.0.1:6800").is_ok()
}

fn start_aria2_if_needed() {
    if is_aria2_running() {
        return;
    }
    let aria2_path = find_aria2c();
    let conf_path = aria2_conf_path();
    log_app(&format!("Starting aria2c from {:?}", aria2_path));

    let mut cmd = Command::new(&aria2_path);
    if conf_path.is_file() {
        cmd.arg(format!("--conf-path={}", conf_path.to_string_lossy()));
    } else {
        // Minimal settings so the UI can talk to aria2 even without a config file.
        log_app(&format!(
            "No aria2.conf at {:?}, using built-in defaults",
            conf_path
        ));
        cmd.args([
            "--enable-rpc=true",
            "--rpc-listen-port=6800",
            "--rpc-listen-all=false",
            "--rpc-allow-origin-all=true",
            "--continue=true",
        ]);
        cmd.arg(format!(
            "--dir={}",
            default_download_dir().to_string_lossy()
        ));
    }
    if conf_value("rpc-secret").is_none() {
        let secret = rpc_secret();
        if !secret.is_empty() {
            cmd.arg(format!("--rpc-secret={}", secret));
        }
    }

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    if let Err(e) = cmd.spawn() {
        log_app(&format!("Failed to start aria2c: {:?}", e));
    }
}

fn show_main_window(app: &AppHandle) {
    let app_clone = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(window) = app_clone.get_webview_window("main") {
            log_app("show_main_window: showing existing 'main' window");
            let icon_bytes = include_bytes!("../icons/128x128.png");
            if let Ok(icon) = tauri::image::Image::from_bytes(icon_bytes) {
                let _ = window.set_icon(icon);
            }
            let _ = window.show();
            let _ = window.unminimize();
            let _ = window.set_focus();
            #[cfg(windows)]
            unsafe {
                if let Ok(hwnd) = window.hwnd() {
                    use windows_sys::Win32::UI::WindowsAndMessaging::{
                        BringWindowToTop, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
                    };
                    ShowWindow(hwnd.0 as _, SW_SHOW);
                    ShowWindow(hwnd.0 as _, SW_RESTORE);
                    BringWindowToTop(hwnd.0 as _);
                    SetForegroundWindow(hwnd.0 as _);
                }
            }
        } else {
            log_app("show_main_window: 'main' window not found, creating it via WebviewWindowBuilder...");
            match tauri::WebviewWindowBuilder::new(&app_clone, "main", tauri::WebviewUrl::App("index.html".into()))
                .title("Kestrel Download Manager")
                .inner_size(840.0, 570.0)
                .min_inner_size(720.0, 480.0)
                .center()
                .build()
            {
                Ok(w) => {
                    log_app("show_main_window: new window created successfully");
                    let icon_bytes = include_bytes!("../icons/128x128.png");
                    if let Ok(icon) = tauri::image::Image::from_bytes(icon_bytes) {
                        let _ = w.set_icon(icon);
                    }
                    let _ = w.show();
                    let _ = w.set_focus();
                }
                Err(e) => log_app(&format!("show_main_window: failed to create window: {:?}", e)),
            }
        }
    });
}

/// 16×16 tray-menu glyph. Windows only paints `hbmpItem` when the menu
/// style includes `MNS_CHECKORBMP`, which muda does not set.
fn tray_glyph(draw: impl Fn(&mut TrayGlyph)) -> Image<'static> {
    let mut glyph = TrayGlyph::new();
    draw(&mut glyph);
    glyph.into_image()
}

struct TrayGlyph {
    buf: Vec<u8>,
}

impl TrayGlyph {
    fn new() -> Self {
        Self {
            buf: vec![0; 16 * 16 * 4],
        }
    }

    fn px(&mut self, x: i32, y: i32, color: [u8; 3]) {
        if (0..16).contains(&x) && (0..16).contains(&y) {
            let i = ((y * 16 + x) * 4) as usize;
            self.buf[i] = color[0];
            self.buf[i + 1] = color[1];
            self.buf[i + 2] = color[2];
            self.buf[i + 3] = 255;
        }
    }

    fn rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 3]) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.px(x, y, color);
            }
        }
    }

    fn frame(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 3]) {
        for x in x0..=x1 {
            self.px(x, y0, color);
            self.px(x, y1, color);
        }
        for y in y0..=y1 {
            self.px(x0, y, color);
            self.px(x1, y, color);
        }
    }

    fn into_image(self) -> Image<'static> {
        Image::new_owned(self.buf, 16, 16)
    }
}

fn tray_icon_open() -> Image<'static> {
    const C: [u8; 3] = [56, 189, 248];
    tray_glyph(|g| {
        g.frame(2, 3, 13, 12, C);
        g.rect(2, 3, 13, 5, C);
        g.rect(4, 7, 7, 8, C);
        g.rect(4, 10, 10, 10, C);
    })
}

fn tray_icon_folder() -> Image<'static> {
    const C: [u8; 3] = [251, 191, 36];
    tray_glyph(|g| {
        g.rect(2, 4, 7, 6, C);
        g.frame(2, 6, 13, 12, C);
        g.rect(3, 7, 12, 7, C);
    })
}

fn tray_icon_pause() -> Image<'static> {
    const C: [u8; 3] = [251, 191, 36];
    tray_glyph(|g| {
        g.rect(4, 3, 6, 12, C);
        g.rect(9, 3, 11, 12, C);
    })
}

fn tray_icon_resume() -> Image<'static> {
    const C: [u8; 3] = [52, 211, 153];
    tray_glyph(|g| {
        for row in 0..10 {
            let inset = row.min(9 - row);
            g.rect(4, 3 + row, 4 + 2 + inset, 3 + row, C);
        }
    })
}

fn tray_icon_quit() -> Image<'static> {
    const C: [u8; 3] = [248, 113, 113];
    tray_glyph(|g| {
        for i in 3..13 {
            g.px(i, i, C);
            g.px(i, 15 - i, C);
            g.px(i + 1, i, C);
            g.px(i + 1, 15 - i, C);
        }
    })
}

#[cfg(windows)]
fn enable_tray_menu_icons<R: tauri::Runtime>(menu: &Menu<R>) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetMenuInfo, SetMenuInfo, MENUINFO, MIM_STYLE, MNS_CHECKORBMP,
    };
    let Ok(handle) = menu.hpopupmenu() else {
        return;
    };
    unsafe {
        let mut info: MENUINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MENUINFO>() as u32;
        info.fMask = MIM_STYLE;
        if GetMenuInfo(handle as _, &mut info) == 0 {
            info.dwStyle = 0;
        }
        info.cbSize = std::mem::size_of::<MENUINFO>() as u32;
        info.fMask = MIM_STYLE;
        info.dwStyle |= MNS_CHECKORBMP;
        let _ = SetMenuInfo(handle as _, &info);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    log_app("run() called");
    std::panic::set_hook(Box::new(|info| {
        log_app(&format!("PANIC IN RUN: {:?}", info));
    }));
    log_app("Calling start_aria2_if_needed");
    start_aria2_if_needed();
    start_extension_secret_server();
    log_app("start_aria2_if_needed done");

    let minimize_flag = Arc::new(AtomicBool::new(true));
    let min_clone = minimize_flag.clone();

    log_app("Building tauri context...");
    let result = tauri::Builder::default()
        .manage(AppState {
            minimize_to_tray: minimize_flag,
        })
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            log_app("setup() closure entered");
            let windows = app.webview_windows();
            log_app(&format!("setup: initial webview_windows count = {}", windows.len()));
            show_main_window(app.handle());

            let show_i = IconMenuItem::with_id(
                app,
                "show",
                "&Open Kestrel Download Manager",
                true,
                Some(tray_icon_open()),
                None::<&str>,
            )?;
            let folder_i = IconMenuItem::with_id(
                app,
                "open_folder",
                "Open &Downloads Folder",
                true,
                Some(tray_icon_folder()),
                None::<&str>,
            )?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let pause_i = IconMenuItem::with_id(
                app,
                "pause_all",
                "&Pause All Transfers",
                true,
                Some(tray_icon_pause()),
                None::<&str>,
            )?;
            let resume_i = IconMenuItem::with_id(
                app,
                "resume_all",
                "&Resume All Transfers",
                true,
                Some(tray_icon_resume()),
                None::<&str>,
            )?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit_i = IconMenuItem::with_id(
                app,
                "quit",
                "&Quit Kestrel",
                true,
                Some(tray_icon_quit()),
                None::<&str>,
            )?;
            let menu = Menu::with_items(app, &[&show_i, &folder_i, &sep1, &pause_i, &resume_i, &sep2, &quit_i])?;
            #[cfg(windows)]
            enable_tray_menu_icons(&menu);

            if let Some(window) = app.get_webview_window("main") {
                let icon_bytes = include_bytes!("../icons/128x128.png");
                if let Ok(icon) = tauri::image::Image::from_bytes(icon_bytes) {
                    let _ = window.set_icon(icon);
                }
            }

            let mut builder = TrayIconBuilder::with_id("main-tray")
                .menu(&menu)
                .tooltip("Kestrel Download Manager")
                .show_menu_on_left_click(false)
                .on_menu_event(|app: &AppHandle, event| match event.id.as_ref() {
                    "show" => {
                        log_app("Tray menu: 'Open Kestrel' clicked");
                        show_main_window(app);
                    }
                    "quit" => {
                        log_app("Tray menu: 'Quit' clicked");
                        app.exit(0);
                    }
                    "open_folder" => {
                        if let Err(err) = open_in_explorer(
                            configured_download_dir().to_string_lossy().to_string(),
                        ) {
                            log_app(&format!("Tray open folder failed: {err}"));
                        }
                    }
                    "pause_all" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.eval("if (window.pauseAllTransfers) window.pauseAllTransfers();");
                        }
                    }
                    "resume_all" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.eval("if (window.resumeAllTransfers) window.resumeAllTransfers();");
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray: &TrayIcon, event| {
                    match event {
                        TrayIconEvent::Click {
                            button: MouseButton::Left,
                            ..
                        }
                        | TrayIconEvent::DoubleClick {
                            button: MouseButton::Left,
                            ..
                        } => {
                            log_app("Tray icon left click/double click");
                            let app = tray.app_handle();
                            show_main_window(app);
                        }
                        _ => {}
                    }
                });

            let icon_bytes = include_bytes!("../icons/32x32.png");
            match tauri::image::Image::from_bytes(icon_bytes) {
                Ok(icon) => {
                    builder = builder.icon(icon);
                }
                Err(e) => {
                    log_app(&format!("Failed to parse icon bytes: {:?}", e));
                    if let Some(icon) = app.default_window_icon() {
                        builder = builder.icon(icon.clone());
                    }
                }
            }

            match builder.build(app) {
                Ok(_) => log_app("TrayIcon successfully built"),
                Err(e) => log_app(&format!("TrayIconBuilder::build failed: {:?}", e)),
            }

            #[cfg(windows)]
            {
                use windows_sys::Win32::Foundation::{CloseHandle, WAIT_OBJECT_0};
                use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

                let event_name: Vec<u16> = "Local\\KestrelAppRestoreEvent\0".encode_utf16().collect();
                let restore_event = unsafe {
                    CreateEventW(std::ptr::null(), 0, 0, event_name.as_ptr())
                };

                if !restore_event.is_null() {
                    let handle = app.handle().clone();
                    let event_usize = restore_event as usize;
                    std::thread::spawn(move || {
                        let event = event_usize as windows_sys::Win32::Foundation::HANDLE;
                        log_app("Restore event listener thread started.");
                        loop {
                            let res = unsafe { WaitForSingleObject(event, 0xFFFFFFFF) };
                            if res == WAIT_OBJECT_0 {
                                log_app("Received secondary instance restore signal! Restoring window...");
                                show_main_window(&handle);
                            } else {
                                log_app(&format!("WaitForSingleObject returned {:#X}, exiting listener thread", res));
                                break;
                            }
                        }
                        unsafe { CloseHandle(event); }
                    });
                } else {
                    log_app("Failed to create restore event in setup()");
                }
            }

            log_app("setup() completed successfully");
            Ok(())
        })
        .on_window_event(move |window, event| {
            log_app(&format!("Window event on {:?}: {:?}", window.label(), event));
            if let WindowEvent::CloseRequested { api, .. } = event {
                if min_clone.load(Ordering::SeqCst) {
                    api.prevent_close();
                    let _ = window.hide();
                    log_app("Window closed: minimized to tray");
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            open_in_explorer,
            open_extension_dir,
            open_downloaded_file,
            open_browser_url,
            check_file_exists,
            delete_download_file,
            rename_download_file,
            check_is_html,
            resolve_web_download_url,
            save_default_dir,
            save_connections_per_server,
            get_rpc_secret,
            set_minimize_to_tray,
            get_minimize_to_tray,
            get_clipboard_text,
            select_directory,
            notify_download_complete,
            update_tray_status,
            window_minimize,
            window_toggle_maximize,
            sync_window_mode,
            window_close,
            set_translucent
        ])
        .run(tauri::generate_context!());

    if let Err(e) = result {
        log_app(&format!("Tauri run error: {:?}", e));
    } else {
        log_app("Tauri run finished normally");
    }
}
