#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{Manager, RunEvent};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

struct ServerState {
    url: String,
    child: Mutex<Option<CommandChild>>,
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .unwrap_or(43127)
}

fn database_url(path: &Path) -> String {
    format!("file:{}", path.to_string_lossy().replace('\\', "/"))
}

fn copy_dir(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// libsql loads its N-API addon with a dynamic require that Bun's compiler cannot
/// embed, so the addon is staged next to the app data and resolved from there.
fn stage_native_modules(app: &tauri::AppHandle, data_dir: &Path) -> Result<(), String> {
    let destination = data_dir.join("node_modules");
    if destination.join("@libsql").exists() {
        return Ok(());
    }

    let mut candidates = Vec::new();
    if let Ok(resource_dir) = app.path().resource_dir() {
        candidates.push(resource_dir.join("native_modules"));
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join("native_modules"),
    );

    let source = candidates
        .into_iter()
        .find(|candidate| candidate.exists())
        .ok_or_else(|| "The bundled libsql native addon is missing.".to_string())?;

    copy_dir(&source, &destination)
        .map_err(|error| format!("Could not stage the libsql native addon: {error}"))
}

fn spawn_sidecar(app: &tauri::AppHandle, port: u16) -> Result<CommandChild, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve the app data folder: {error}"))?;

    fs::create_dir_all(&data_dir)
        .map_err(|error| format!("Could not create the app data folder: {error}"))?;

    stage_native_modules(app, &data_dir)?;

    let db_path: PathBuf = data_dir.join("treasurer.db");
    let cors_origin = if cfg!(debug_assertions) {
        "http://localhost:1420"
    } else {
        "http://tauri.localhost,tauri://localhost"
    };

    let command = app
        .shell()
        .sidecar("server")
        .map_err(|error| format!("Could not locate the bundled server: {error}"))?
        .env("PORT", port.to_string())
        .env("DATABASE_URL", database_url(&db_path))
        .env("CORS_ORIGIN", cors_origin)
        .env("NODE_ENV", "production")
        .current_dir(&data_dir);

    let (mut receiver, child) = command
        .spawn()
        .map_err(|error| format!("Could not start the local server: {error}"))?;

    tauri::async_runtime::spawn(async move {
        while let Some(event) = receiver.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    println!("[server] {}", String::from_utf8_lossy(&line));
                }
                CommandEvent::Stderr(line) => {
                    eprintln!("[server] {}", String::from_utf8_lossy(&line));
                }
                CommandEvent::Error(error) => {
                    eprintln!("[server] {error}");
                }
                CommandEvent::Terminated(payload) => {
                    eprintln!("[server] exited with {:?}", payload.code);
                }
                _ => {}
            }
        }
    });

    Ok(child)
}

#[tauri::command]
fn server_url(state: tauri::State<'_, ServerState>) -> String {
    state.url.clone()
}

#[tauri::command]
fn read_text_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|error| format!("Could not read {path}: {error}"))
}

#[tauri::command]
fn write_text_file(path: String, contents: String) -> Result<(), String> {
    fs::write(&path, contents).map_err(|error| format!("Could not write {path}: {error}"))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let port = free_port();
            let child = spawn_sidecar(app.handle(), port)?;

            app.manage(ServerState {
                url: format!("http://127.0.0.1:{port}"),
                child: Mutex::new(Some(child)),
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            server_url,
            read_text_file,
            write_text_file
        ])
        .build(tauri::generate_context!())
        .expect("error while building the treasurer app")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                if let Some(state) = app.try_state::<ServerState>() {
                    if let Ok(mut guard) = state.child.lock() {
                        if let Some(child) = guard.take() {
                            let _ = child.kill();
                        }
                    }
                }
            }
        });
}
