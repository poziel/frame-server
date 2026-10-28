//! Host-independent PTY sessions. Hosts assign trusted owner identities.
mod killer;
mod profiles;
use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};

/// Host-facing operation result; messages contain no authentication material.
pub type Result<T> = std::result::Result<T, String>;
type Sessions = Arc<Mutex<HashMap<String, Session>>>;
#[derive(Default)]
/// Owns terminal sessions; dropping it terminates remaining shell processes.
pub struct Terminals(Sessions);
struct Session {
    owner: String,
    master: Box<dyn MasterPty + Send>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}
#[derive(Deserialize)]
/// Explicit shell profile, working directory, and initial character dimensions.
pub struct SpawnOptions {
    pub profile: String,
    pub cwd: Option<String>,
    pub cols: u16,
    pub rows: u16,
}
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
/// Raw terminal bytes and process lifecycle notifications.
pub enum Output {
    Data { data: Vec<u8> },
    Exit { code: u32 },
    Error { message: String },
}
static NEXT: AtomicU64 = AtomicU64::new(1);
fn shell_command(path: impl AsRef<std::ffi::OsStr>) -> CommandBuilder {
    let mut command = CommandBuilder::new(path);
    // Inherit the host's resolved environment. portable-pty otherwise overlays raw
    // registry values on Windows, which can reintroduce unresolved placeholders.
    command.env_clear();
    for (name, value) in std::env::vars_os() {
        command.env(name, value);
    }
    command
}
fn size(cols: u16, rows: u16) -> Result<PtySize> {
    if cols == 0 || rows == 0 || cols > 1000 || rows > 1000 {
        return Err("Terminal dimensions must be between 1 and 1000".into());
    }
    Ok(PtySize {
        cols,
        rows,
        pixel_width: 0,
        pixel_height: 0,
    })
}
fn owned<'a>(
    sessions: &'a mut HashMap<String, Session>,
    id: &str,
    owner: &str,
) -> Result<&'a mut Session> {
    sessions
        .get_mut(id)
        .filter(|s| s.owner == owner)
        .ok_or_else(|| "Terminal session not found".into())
}

/// Receives bytes and lifecycle events; return an error when disconnected.
pub type OutputSink = Arc<dyn Fn(Output) -> Result<()> + Send + Sync>;

impl Terminals {
    /// Detect available shell profiles without starting a terminal.
    pub fn profiles(&self) -> Vec<profiles::Profile> {
        profiles::detect()
    }

    /// Start a PTY for a trusted host-assigned owner and event receiver.
    pub fn spawn(&self, owner: &str, options: SpawnOptions, output: OutputSink) -> Result<String> {
        let sessions = self.0.clone();
        let owner = owner.to_string();
        let profile = profiles::detect()
            .into_iter()
            .find(|p| p.id == options.profile)
            .ok_or("Unknown terminal profile")?;
        let pair = native_pty_system()
            .openpty(size(options.cols, options.rows)?)
            .map_err(|e| e.to_string())?;
        let mut command = shell_command(profile.path);
        if profile.id == "git-bash" {
            command.args(["--login", "-i"]);
        }
        if let Some(cwd) = options.cwd {
            if !std::path::Path::new(&cwd).is_dir() {
                return Err("Working directory does not exist".into());
            }
            command.cwd(cwd);
        } else if let Some(home) =
            std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
        {
            command.cwd(home);
        }
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env("TERM_PROGRAM", "Frame");
        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = Arc::new(Mutex::new(
            pair.master.take_writer().map_err(|e| e.to_string())?,
        ));
        let starting = Arc::new(AtomicBool::new(true));
        let reader_starting = starting.clone();
        let startup_writer = writer.clone();
        let disconnected = Arc::new(AtomicBool::new(false));
        let reader_disconnected = disconnected.clone();
        let id = NEXT.fetch_add(1, Ordering::Relaxed).to_string();
        let stream = output.clone();
        let reader_sessions = sessions.clone();
        let reader_id = id.clone();
        let reader_thread = std::thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            let mut tail = Vec::new();
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        // ConPTY can request the initial cursor position before CreateProcess
                        // returns. The frontend renderer cannot exist until spawn resolves.
                        if reader_starting.load(Ordering::Acquire) {
                            tail.extend_from_slice(&buffer[..n]);
                            if tail.windows(4).any(|w| w == b"\x1b[6n") {
                                if let Ok(mut writer) = startup_writer.lock() {
                                    let _ = writer.write_all(b"\x1b[1;1R");
                                    let _ = writer.flush();
                                }
                            }
                            if tail.len() > 3 {
                                tail.drain(..tail.len() - 3);
                            }
                        }
                        if stream(Output::Data {
                            data: buffer[..n].to_vec(),
                        })
                        .is_err()
                        {
                            reader_disconnected.store(true, Ordering::Release);
                            // A disconnected receiver must not leave an orphan shell.
                            if let Ok(mut map) = reader_sessions.lock() {
                                if let Some(session) = map.get_mut(&reader_id) {
                                    let _ = session.killer.kill();
                                }
                            }
                            break;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        });
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| e.to_string())?;
        starting.store(false, Ordering::Release);
        drop(pair.slave);
        let mut killer = match killer::clone_killer(child.as_ref()) {
            Ok(killer) => killer,
            Err(error) => {
                let _ = child.kill();
                return Err(error.to_string());
            }
        };
        let mut map = sessions.lock().map_err(|e| e.to_string())?;
        if disconnected.load(Ordering::Acquire) {
            drop(map);
            let _ = killer.kill();
            let _ = child.wait();
            return Err("Terminal receiver closed during startup".into());
        }
        map.insert(
            id.clone(),
            Session {
                owner,
                master: pair.master,
                writer,
                killer,
            },
        );
        drop(map);
        let wait_id = id.clone();
        std::thread::spawn(move || {
            let status = child.wait();
            let removed = sessions
                .lock()
                .ok()
                .and_then(|mut map| map.remove(&wait_id));
            drop(removed);
            let _ = reader_thread.join();
            let event = match status {
                Ok(status) => Output::Exit {
                    code: status.exit_code(),
                },
                Err(e) => Output::Error {
                    message: e.to_string(),
                },
            };
            let _ = output(event);
        });
        Ok(id)
    }

    /// Write at most 1 MiB, rejecting sessions owned by another host scope.
    pub fn write(&self, owner: &str, id: &str, data: &str) -> Result<()> {
        if data.len() > 1024 * 1024 {
            return Err("Paste exceeds 1 MiB; paste smaller chunks".into());
        }
        let sessions = self.0.clone();
        let writer = {
            let mut map = sessions.lock().map_err(|e| e.to_string())?;
            owned(&mut map, &id, &owner)?.writer.clone()
        };
        let mut writer = writer.lock().map_err(|e| e.to_string())?;
        writer
            .write_all(data.as_bytes())
            .and_then(|_| writer.flush())
            .map_err(|e| e.to_string())
    }

    /// Resize a live terminal after validating dimensions and ownership.
    pub fn resize(&self, owner: &str, id: &str, cols: u16, rows: u16) -> Result<()> {
        let mut map = self.0.lock().map_err(|e| e.to_string())?;
        owned(&mut map, id, owner)?
            .master
            .resize(size(cols, rows)?)
            .map_err(|e| e.to_string())
    }

    /// Close a terminal; closing an already removed session is harmless.
    pub fn close(&self, owner: &str, id: &str) -> Result<()> {
        let sessions = self.0.clone();
        let mut map = sessions.lock().map_err(|e| e.to_string())?;
        // An already exited session can still be dismissed in the UI.
        if !map.contains_key(id) {
            return Ok(());
        }
        owned(&mut map, &id, &owner)?
            .killer
            .kill()
            .map_err(|e| e.to_string())?;
        let removed = map.remove(id);
        drop(map);
        drop(removed);
        Ok(())
    }

    /// Terminate every session for one trusted owner during host cleanup.
    pub fn close_owner(&self, owner: &str) {
        self.drain(Some(owner));
    }

    fn drain(&self, owner: Option<&str>) {
        let removed = self
            .0
            .lock()
            .map(|mut map| {
                let ids: Vec<_> = map
                    .iter()
                    .filter(|(_, s)| owner.is_none() || owner == Some(s.owner.as_str()))
                    .map(|(id, _)| id.clone())
                    .collect();
                ids.into_iter()
                    .filter_map(|id| map.remove(&id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for mut session in removed {
            let _ = session.killer.kill();
        }
    }
}

impl Drop for Terminals {
    fn drop(&mut self) {
        self.drain(None);
    }
}

pub use profiles::Profile;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_real_profiles_and_one_default() {
        let profiles = profiles::detect();
        assert!(!profiles.is_empty());
        assert_eq!(profiles.iter().filter(|p| p.default).count(), 1);
        assert!(profiles.iter().all(|p| p.path.is_file()));
    }
    #[test]
    fn rejects_invalid_dimensions() {
        assert!(size(0, 24).is_err());
        assert!(size(80, 1001).is_err());
        assert!(size(120, 40).is_ok());
    }
    #[test]
    fn native_pty_streams_shell_output() {
        check_native_pty(false);
    }
    #[test]
    fn native_pty_closes_running_shell() {
        check_native_pty(true);
    }
    fn check_native_pty(terminate: bool) {
        let pair = native_pty_system().openpty(size(80, 24).unwrap()).unwrap();
        let profile = profiles::detect()
            .into_iter()
            .find(|p| p.id == if cfg!(windows) { "cmd" } else { "sh" })
            .unwrap();
        let mut command = shell_command(profile.path);
        if cfg!(windows) {
            command.args([
                "/D",
                if terminate { "/K" } else { "/C" },
                "echo",
                "FRAME_PTY_OK",
            ]);
        } else {
            command.args([
                "-c",
                if terminate {
                    "printf FRAME_PTY_OK; sleep 30"
                } else {
                    "printf FRAME_PTY_OK"
                },
            ]);
        }
        let mut reader = pair.master.try_clone_reader().unwrap();
        let mut writer = pair.master.take_writer().unwrap();
        let (send, receive) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            let mut tail = Vec::new();
            while let Ok(n) = reader.read(&mut buffer) {
                if n == 0 {
                    break;
                }
                tail.extend_from_slice(&buffer[..n]);
                if tail.windows(4).any(|w| w == b"\x1b[6n") {
                    let _ = writer.write_all(b"\x1b[1;1R");
                    let _ = writer.flush();
                }
                if tail.len() > 3 {
                    tail.drain(..tail.len() - 3);
                }
                if send.send(buffer[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
        let mut child = pair.slave.spawn_command(command).unwrap();
        drop(pair.slave);
        let mut bytes = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !String::from_utf8_lossy(&bytes).contains("FRAME_PTY_OK") {
            let chunk =
                receive.recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()));
            match chunk {
                Ok(chunk) => {
                    bytes.extend_from_slice(&chunk);
                }
                Err(error) => {
                    let _ = child.kill();
                    panic!(
                        "PTY output timed out: {error}; output={:?}",
                        String::from_utf8_lossy(&bytes)
                    );
                }
            }
        }
        if terminate {
            killer::clone_killer(child.as_ref())
                .unwrap()
                .kill()
                .unwrap();
        }
        assert_eq!(child.wait().unwrap().success(), !terminate);
        drop(pair.master);
        thread.join().unwrap();
        assert!(String::from_utf8_lossy(&bytes).contains("FRAME_PTY_OK"));
    }
}

#[cfg(feature = "tauri")]
pub mod tauri;
