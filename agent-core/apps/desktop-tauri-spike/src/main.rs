mod terminal;

use std::collections::HashMap;
use tauri::{WebviewUrl, WebviewWindowBuilder};

fn main() {
    tauri::Builder::default()
        .manage(terminal::TerminalMap::default())
        .invoke_handler(tauri::generate_handler![
            terminal::terminal_start,
            terminal::terminal_write,
            terminal::terminal_resize,
            terminal::terminal_dispose,
        ])
        .setup(|app| {
            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Hermes PTY Spike")
                .inner_size(900.0, 600.0)
                .min_inner_size(400.0, 300.0)
                .build()?;
            let _ = window;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
