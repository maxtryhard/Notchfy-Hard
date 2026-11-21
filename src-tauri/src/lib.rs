mod media;
mod platform;
mod store;
mod tokens;

use std::sync::Mutex;

use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, LogicalPosition, LogicalSize, Manager, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

const ISLAND: &str = "island";
const EXPANDED_WIDTH: f64 = 680.0;
const EXPANDED_HEIGHT: f64 = 300.0;
const PILL_WIDTH: f64 = 220.0;
const PILL_HEIGHT: f64 = 32.0;
/// Largura de cada "asa" que aparece ao lado do notch físico (capa do álbum à
/// esquerda, hora à direita).
const NOTCH_WING: f64 = 64.0;

/// Geometria da ilha. O frontend usa estes valores para desenhar a pílula
/// exatamente do tamanho da janela recolhida.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Layout {
    has_notch: bool,
    notch_width: f64,
    collapsed_width: f64,
    collapsed_height: f64,
    expanded_width: f64,
    expanded_height: f64,
    platform: &'static str,
}

impl Layout {
    fn new(notch: Option<platform::Notch>) -> Self {
        let (has_notch, notch_width, collapsed_width, collapsed_height) = match notch {
            Some(n) => (true, n.width, n.width + 2.0 * NOTCH_WING, n.height),
            None => (false, 0.0, PILL_WIDTH, PILL_HEIGHT),
        };
        Self {
            has_notch,
            notch_width,
            collapsed_width,
            collapsed_height,
            expanded_width: EXPANDED_WIDTH.max(collapsed_width),
            expanded_height: EXPANDED_HEIGHT,
            platform: std::env::consts::OS,
        }
    }
}

struct AppState {
    layout: Layout,
    store: Mutex<store::Store>,
    tokens: tokens::Cache,
    drag_icon: Option<String>,
}

/// Centraliza a janela no topo do monitor principal com o tamanho pedido.
fn place(window: &WebviewWindow, width: f64, height: f64) -> tauri::Result<()> {
    let monitor = match window.primary_monitor()? {
        Some(m) => m,
        None => match window.current_monitor()? {
            Some(m) => m,
            None => return Ok(()),
        },
    };
    let scale = monitor.scale_factor();
    let origin = monitor.position().to_logical::<f64>(scale);
    let size = monitor.size().to_logical::<f64>(scale);
    window.set_size(LogicalSize::new(width, height))?;
    window.set_position(LogicalPosition::new(
        origin.x + ((size.width - width) / 2.0).round(),
        origin.y,
    ))?;
    Ok(())
}

#[tauri::command]
fn get_layout(state: State<'_, AppState>) -> Layout {
    state.layout
}

#[tauri::command]
fn set_expanded(window: WebviewWindow, state: State<'_, AppState>, expanded: bool) -> Result<(), String> {
    let l = state.layout;
    let (w, h) = if expanded {
        (l.expanded_width, l.expanded_height)
    } else {
        (l.collapsed_width, l.collapsed_height)
    };
    #[cfg(debug_assertions)]
    eprintln!("notchfy: set_expanded({expanded}) -> {w}x{h}");
    place(&window, w, h).map_err(|e| e.to_string())
}

#[tauri::command]
async fn media_state() -> media::MediaState {
    tauri::async_runtime::spawn_blocking(media::state)
        .await
        .unwrap_or_default()
}

#[tauri::command]
async fn media_control(action: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || media::control(&action))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn ai_usage(app: AppHandle) -> tokens::Report {
    tauri::async_runtime::spawn_blocking(move || tokens::report(&app.state::<AppState>().tokens))
        .await
        .unwrap_or_default()
}

#[tauri::command]
fn get_data(state: State<'_, AppState>) -> store::Data {
    state.store.lock().unwrap().snapshot()
}

#[tauri::command]
fn save_shortcuts(state: State<'_, AppState>, shortcuts: Vec<store::Shortcut>) -> Result<store::Data, String> {
    let mut s = state.store.lock().unwrap();
    s.data.shortcuts = shortcuts;
    s.save()?;
    Ok(s.snapshot())
}

#[tauri::command]
fn launch_shortcut(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    let shortcut = state
        .store
        .lock()
        .unwrap()
        .data
        .shortcuts
        .iter()
        .find(|s| s.id == id)
        .cloned()
        .ok_or("Atalho não encontrado")?;
    store::launch(&app, &shortcut)
}

#[tauri::command]
fn shelf_add(state: State<'_, AppState>, paths: Vec<String>) -> Result<store::Data, String> {
    let mut s = state.store.lock().unwrap();
    s.shelf_add(paths);
    s.save()?;
    Ok(s.snapshot())
}

#[tauri::command]
fn shelf_remove(state: State<'_, AppState>, path: Option<String>) -> Result<store::Data, String> {
    let mut s = state.store.lock().unwrap();
    match path {
        Some(p) => s.data.shelf.retain(|i| i.path != p),
        None => s.data.shelf.clear(),
    }
    s.save()?;
    Ok(s.snapshot())
}

#[tauri::command]
fn open_path(app: AppHandle, path: String, reveal: bool) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    if reveal {
        app.opener().reveal_item_in_dir(&path)
    } else {
        app.opener().open_path(&path, None::<&str>)
    }
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn drag_icon(state: State<'_, AppState>) -> Option<String> {
    state.drag_icon.clone()
}

/// O plugin de arrastar exige um ícone em disco; gravamos um no cache.
fn write_drag_icon(app: &AppHandle) -> Option<String> {
    let dir = app.path().app_cache_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join("drag-icon.png");
    std::fs::write(&path, include_bytes!("../icons/64x64.png")).ok()?;
    Some(path.to_string_lossy().into_owned())
}

/// A janela é criada aqui, e não no tauri.conf.json, porque o `resizable`
/// precisa variar por sistema (ver `platform::configure_window`).
fn create_island(app: &AppHandle, layout: Layout) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, ISLAND, WebviewUrl::default())
        .title("Notchfy")
        .inner_size(layout.collapsed_width, layout.collapsed_height)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(cfg!(target_os = "linux"))
        .maximizable(false)
        .minimizable(false)
        .shadow(false)
        .focused(false)
        .visible(false)
        .visible_on_all_workspaces(true)
        .transparent(true)
        // macOS: o primeiro clique já aciona o botão, sem antes focar a janela.
        .accept_first_mouse(true)
        .build()
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "Mostrar/ocultar Notchfy", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &quit])?;
    let mut tray = TrayIconBuilder::with_id("notchfy")
        .tooltip("Notchfy")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "toggle" => {
                if let Some(w) = app.get_webview_window(ISLAND) {
                    let _ = if w.is_visible().unwrap_or(false) { w.hide() } else { w.show() };
                }
            }
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

pub fn run() {
    platform::prepare_environment();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_drag::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // A detecção do notch roda aqui porque o setup executa na thread
            // principal, exigida pelo AppKit.
            let layout = Layout::new(platform::detect_notch());
            let handle = app.handle().clone();
            let config_dir = handle.path().app_config_dir()?;
            app.manage(AppState {
                layout,
                store: Mutex::new(store::Store::load(config_dir.join("notchfy.json"))),
                tokens: tokens::Cache::default(),
                drag_icon: write_drag_icon(&handle),
            });

            if let Err(e) = build_tray(&handle) {
                eprintln!("notchfy: bandeja indisponível: {e}");
            }

            let window = create_island(&handle, layout)?;
            platform::configure_window(&window);
            place(&window, layout.collapsed_width, layout.collapsed_height)?;
            window.show()?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_layout,
            set_expanded,
            media_state,
            media_control,
            ai_usage,
            get_data,
            save_shortcuts,
            launch_shortcut,
            shelf_add,
            shelf_remove,
            open_path,
            drag_icon,
        ])
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o Notchfy");
}
