//! Manages the private `dbx-web` process used by the desktop shell.
//!
//! Development runs start the backend with `dev.sh`; packaged builds carry a
//! sidecar next to the app resources and start it on demand.

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};

pub(crate) const API: &str = "http://127.0.0.1:4225";

const STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
const POLL_INTERVAL: Duration = Duration::from_millis(150);

/// Keeps a backend that this process started alive for as long as the app is
/// running. A backend that was already running (for example via `dev.sh`) is
/// reused and deliberately left alone on exit.
#[derive(Clone)]
pub(crate) struct Backend {
    process: Rc<BackendProcess>,
}

struct BackendProcess {
    child: std::cell::RefCell<Option<Child>>,
}

impl Backend {
    pub(crate) fn start() -> Result<Self, String> {
        if backend_is_ready() {
            return Ok(Self::from_child(None));
        }

        let binary = sidecar_path().ok_or_else(|| {
            "dbx-web sidecar not found; reinstall OMDBX or set DBX_WEB_BIN".to_string()
        })?;
        let data_dir = app_data_dir().join("dbx-web");
        fs::create_dir_all(&data_dir)
            .map_err(|error| format!("failed to create backend data directory: {error}"))?;

        let log_dir = app_data_dir().join("logs");
        fs::create_dir_all(&log_dir)
            .map_err(|error| format!("failed to create log directory: {error}"))?;
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("dbx-web.log"))
            .map_err(|error| format!("failed to open backend log: {error}"))?;
        let stderr = log
            .try_clone()
            .map_err(|error| format!("failed to duplicate backend log handle: {error}"))?;

        let mut command = Command::new(&binary);
        command
            .env("DBX_PORT", "4225")
            .env("DBX_BIND_ADDR", "127.0.0.1")
            .env("DBX_DISABLE_PASSWORD", "1")
            .env("DBX_DATA_DIR", &data_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(stderr));

        let mut child = command
            .spawn()
            .map_err(|error| format!("failed to start {}: {error}", binary.display()))?;

        let started_at = Instant::now();
        loop {
            if backend_is_ready() {
                return Ok(Self::from_child(Some(child)));
            }
            if let Some(status) = child
                .try_wait()
                .map_err(|error| format!("failed to inspect backend process: {error}"))?
            {
                return Err(format!(
                    "dbx-web exited during startup ({status}); see {}",
                    log_dir.join("dbx-web.log").display()
                ));
            }
            if started_at.elapsed() >= STARTUP_TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "dbx-web did not become ready within {} seconds; see {}",
                    STARTUP_TIMEOUT.as_secs(),
                    log_dir.join("dbx-web.log").display()
                ));
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn from_child(child: Option<Child>) -> Self {
        Self {
            process: Rc::new(BackendProcess {
                child: std::cell::RefCell::new(child),
            }),
        }
    }

    pub(crate) fn stop(&self) {
        let child = self.process.child.borrow_mut().take();
        stop_child(child);
    }
}

impl Drop for BackendProcess {
    fn drop(&mut self) {
        stop_child(self.child.borrow_mut().take());
    }
}

fn stop_child(child: Option<Child>) {
    let Some(mut child) = child else {
        return;
    };
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}

fn backend_is_ready() -> bool {
    let agent = ureq::AgentBuilder::new()
        .try_proxy_from_env(false)
        .timeout_connect(Duration::from_millis(400))
        .timeout_read(Duration::from_millis(800))
        .build();
    agent
        .get(&format!("{API}/api/version"))
        .call()
        .map(|response| response.status() == 200)
        .unwrap_or(false)
}

fn sidecar_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("DBX_WEB_BIN").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }

    let executable = std::env::current_exe().ok()?;
    let binary_name = if cfg!(windows) {
        "dbx-web.exe"
    } else {
        "dbx-web"
    };
    let executable_dir = executable.parent()?;

    let mut candidates = Vec::new();

    #[cfg(target_os = "macos")]
    if let Some(contents_dir) = executable_dir.parent() {
        candidates.push(contents_dir.join("Resources").join(binary_name));
    }

    candidates.push(executable_dir.join(binary_name));

    if let Some(app_dir) = std::env::var_os("APPDIR").map(PathBuf::from) {
        candidates.push(app_dir.join("usr/lib/omdbx").join(binary_name));
        candidates.push(app_dir.join(binary_name));
    }

    // Running the binary directly from a source checkout should still work.
    if let Some(repo_dir) = executable_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .and_then(Path::parent)
    {
        candidates.push(repo_dir.join("target/release").join(binary_name));
    }

    candidates.into_iter().find(|path| path.is_file())
}

fn app_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        return home.join("Library/Application Support/OMDBX");
    }

    #[cfg(windows)]
    if let Some(roaming) = std::env::var_os("APPDATA").map(PathBuf::from) {
        return roaming.join("OMDBX");
    }

    #[cfg(not(any(target_os = "macos", windows)))]
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from) {
        return data_home.join("omdbx");
    }

    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        #[cfg(windows)]
        return home.join("AppData/Roaming/OMDBX");
        #[cfg(not(windows))]
        return home.join(".local/share/omdbx");
    }

    std::env::temp_dir().join("omdbx")
}
