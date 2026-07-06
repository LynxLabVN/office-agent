use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

/// Shared map of active terminal sessions.
pub type TerminalMap = Arc<Mutex<HashMap<u64, TerminalSession>>>;

/// A single PTY-backed terminal session.
pub struct TerminalSession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

/// Options used to start a new terminal session.
#[derive(Debug, serde::Deserialize)]
pub struct StartOptions {
    #[serde(default = "default_shell")]
    shell: String,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(default = "default_rows")]
    rows: u16,
    #[serde(default = "default_cols")]
    cols: u16,
}

fn default_shell() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| {
        if cfg!(target_os = "windows") {
            "powershell.exe".to_string()
        } else {
            "/bin/sh".to_string()
        }
    })
}

fn default_rows() -> u16 { 24 }
fn default_cols() -> u16 { 80 }

/// Response returned after starting a terminal session.
#[derive(Serialize)]
pub struct StartResponse {
    id: u64,
}

/// Start a new PTY-backed terminal session and begin streaming output.
#[tauri::command]
pub fn terminal_start(
    options: StartOptions,
    app: AppHandle,
    state: State<'_, TerminalMap>,
) -> Result<StartResponse, String> {
    let id = {
        let mut sessions = state.lock().map_err(|e| e.to_string())?;
        // Find next free id.
        let mut id = 1u64;
        while sessions.contains_key(&id) {
            id += 1;
        }
        id
    };

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: options.rows,
            cols: options.cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("failed to open pty: {e}"))?;

    let mut cmd = CommandBuilder::new(&options.shell);
    if let Some(cwd) = options.cwd {
        cmd.cwd(cwd.into());
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("failed to spawn shell: {e}"))?;

    let master = pair.master;
    let writer = master
        .take_writer()
        .map_err(|e| format!("failed to get pty writer: {e}"))?;

    {
        let mut sessions = state.lock().map_err(|e| e.to_string())?;
        sessions.insert(
            id,
            TerminalSession {
                master,
                writer,
                child,
            },
        );
    }

    // Spawn a reader thread that forwards PTY output to the webview.
    let state_for_reader: TerminalMap = Arc::clone(&*state);
    let data_channel = format!("terminal:{id}:data");
    let exit_channel = format!("terminal:{id}:exit");
    std::thread::spawn(move || {
        let mut reader = {
            let sessions = state_for_reader.lock().map_err(|e| e.to_string()).ok()?;
            let session = sessions.get(&id)?;
            session
                .master
                .try_clone_reader()
                .map_err(|e| format!("failed to clone reader: {e}"))
                .ok()?
        };

        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = app.emit(&data_channel, &buf[..n]);
                }
                Err(e) => {
                    let _ = app.emit(&exit_channel, format!("read error: {e}"));
                    break;
                }
            }
        }

        let exit_code = {
            let sessions = state_for_reader.lock().map_err(|e| e.to_string()).ok()?;
            let session = sessions.get(&id)?;
            session
                .child
                .wait()
                .map(|status| status.exit_code().unwrap_or(-1))
                .unwrap_or(-1)
        };

        let _ = app.emit(&exit_channel, exit_code);
        let _ = state_for_reader.lock().map_err(|e| e.to_string()).ok().map(|mut s| s.remove(&id));
        Some(())
    });

    Ok(StartResponse { id })
}

/// Write raw bytes to a terminal session.
#[tauri::command]
pub fn terminal_write(
    id: u64,
    data: String,
    state: State<'_, TerminalMap>,
) -> Result<(), String> {
    let mut sessions = state.lock().map_err(|e| e.to_string())?;
    let session = sessions
        .get_mut(&id)
        .ok_or_else(|| format!("terminal session {id} not found"))?;
    session
        .writer
        .write_all(data.as_bytes())
        .map_err(|e| format!("failed to write: {e}"))?;
    session
        .writer
        .flush()
        .map_err(|e| format!("failed to flush: {e}"))?;
    Ok(())
}

/// Resize a terminal session.
#[tauri::command]
pub fn terminal_resize(
    id: u64,
    rows: u16,
    cols: u16,
    state: State<'_, TerminalMap>,
) -> Result<(), String> {
    let sessions = state.lock().map_err(|e| e.to_string())?;
    let session = sessions
        .get(&id)
        .ok_or_else(|| format!("terminal session {id} not found"))?;
    session
        .master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("failed to resize: {e}"))?;
    Ok(())
}

/// Dispose of a terminal session, killing the child process.
#[tauri::command]
pub fn terminal_dispose(id: u64, state: State<'_, TerminalMap>) -> Result<(), String> {
    let mut sessions = state.lock().map_err(|e| e.to_string())?;
    if let Some(mut session) = sessions.remove(&id) {
        let _ = session.child.kill();
        let _ = session.child.wait();
    }
    Ok(())
}
