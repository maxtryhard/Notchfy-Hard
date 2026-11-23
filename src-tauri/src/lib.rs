mod battery;
mod hover;
mod media;
mod notify;
mod platform;
mod store;
mod tokens;

use std::sync::Mutex;

use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

const ISLAND: &str = "island";
const EXPANDED_WIDTH: f64 = 680.0;
const EXPANDED_HEIGHT: f64 = 320.0;
const PILL_WIDTH: f64 = 220.0;
const PILL_HEIGHT: f64 = 32.0;
/// Largura de cada "asa" que aparece ao lado do notch físico (capa do álbum à
/// esquerda, hora à direita).
const NOTCH_WING: f64 = 64.0;

/// Geometria da ilha. O frontend desenha a pílula e o painel com estes valores,
/// e o detector de hover usa os mesmos para saber onde o mouse conta.
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
    placement: Mutex<Option<Placement>>,
    hover_area: hover::Area,
    notices: Mutex<Option<notify::Listener>>,
}

/// Avisos sobre a própria escuta (ligou, foi recusada, parou), para a ilha
/// mostrar. Não entram na caixa de notificações.
const NOTICE_STATUS_EVENT: &str = "island-notification-status";

#[derive(Clone, Serialize)]
struct NoticeStatus {
    title: &'static str,
    body: String,
}

fn emit_notice_status(app: &AppHandle, title: &'static str, body: String) {
    // emit só falha com o app fechando; não há a quem avisar.
    let _ = app.emit(NOTICE_STATUS_EVENT, NoticeStatus { title, body });
}

/// Liga ou desliga a escuta de notificações e salva a escolha. Se o sistema
/// recusar, a opção fica desligada e o erro explica o motivo.
fn set_notifications(app: &AppHandle, on: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let started = if on {
        let mut listener = state.notices.lock().unwrap();
        if listener.is_none() {
            let (on_notice, on_problem) = (app.clone(), app.clone());
            *listener = Some(notify::start(
                move |notice| {
                    // emit só falha com o app fechando; não há a quem avisar.
                    let _ = on_notice.emit(notify::NOTICE_EVENT, notice);
                },
                move |reason| emit_notice_status(&on_problem, "Notificações pararam", reason),
            )?);
        }
        true
    } else {
        *state.notices.lock().unwrap() = None;
        false
    };
    let mut store = state.store.lock().unwrap();
    store.data.notifications = started;
    store.save()
}

/// Canto superior esquerdo da janela, em pixels lógicos, e a escala do monitor.
#[derive(Clone, Copy)]
struct Placement {
    x: f64,
    y: f64,
    scale: f64,
}

/// A janela tem sempre o tamanho do painel aberto e nunca é redimensionada:
/// redimensionar a cada hover deixava a abertura lenta e piscando no Windows.
/// Abrir e fechar é só animação no CSS; fora da área ativa (ver `active_area`)
/// a janela deixa o mouse passar para o app de baixo.
fn place(window: &WebviewWindow, layout: Layout) -> tauri::Result<Option<Placement>> {
    let monitor = match window.primary_monitor()? {
        Some(m) => m,
        None => match window.current_monitor()? {
            Some(m) => m,
            None => return Ok(None),
        },
    };
    let scale = monitor.scale_factor();
    let origin = monitor.position().to_logical::<f64>(scale);
    let size = monitor.size().to_logical::<f64>(scale);
    let x = origin.x + ((size.width - layout.expanded_width) / 2.0).round();
    window.set_size(LogicalSize::new(
        layout.expanded_width,
        layout.expanded_height,
    ))?;
    window.set_position(LogicalPosition::new(x, origin.y))?;
    Ok(Some(Placement {
        x,
        y: origin.y,
        scale,
    }))
}

/// Onde o mouse conta como "em cima da ilha": a pílula, com ela fechada; o
/// painel inteiro, com ela aberta. Ambos ficam centralizados no topo da janela.
fn active_area(p: Placement, layout: Layout, expanded: bool) -> hover::Rect {
    let (w, h) = if expanded {
        (layout.expanded_width, layout.expanded_height)
    } else {
        (layout.collapsed_width, layout.collapsed_height)
    };
    let left = p.x + (layout.expanded_width - w) / 2.0;
    hover::Rect {
        left: left * p.scale,
        top: p.y * p.scale,
        right: (left + w) * p.scale,
        bottom: (p.y + h) * p.scale,
    }
}

fn zone(p: Placement, layout: Layout, expanded: bool) -> hover::Zone {
    hover::Zone {
        active: active_area(p, layout, expanded),
        window_left: p.x * p.scale,
        window_top: p.y * p.scale,
        scale: p.scale,
        expanded,
    }
}

/// Posiciona a ilha fechada e avisa o detector de hover.
fn show_collapsed(window: &WebviewWindow, state: &AppState) -> tauri::Result<()> {
    let placement = place(window, state.layout)?;
    *state.placement.lock().unwrap() = placement;
    *state.hover_area.lock().unwrap() = placement.map(|p| zone(p, state.layout, false));
    window.show()
}

#[tauri::command]
fn get_layout(state: State<'_, AppState>) -> Layout {
    state.layout
}

#[tauri::command]
fn set_expanded(state: State<'_, AppState>, expanded: bool) {
    if let Some(p) = *state.placement.lock().unwrap() {
        *state.hover_area.lock().unwrap() = Some(zone(p, state.layout, expanded));
    }
}

/// Chamado pela interface ao abrir: retoma a escuta se a opção estava ligada
/// e devolve se ela ficou ligada, para o interruptor começar no estado certo.
#[tauri::command]
fn notifications_init(app: AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    let enabled = state.store.lock().unwrap().data.notifications;
    if enabled {
        enable_notifications(&app, true)?;
    }
    Ok(enabled)
}

/// O interruptor "Mostrar na ilha" da aba Notificações.
#[tauri::command]
fn set_notifications_enabled(app: AppHandle, on: bool) -> Result<(), String> {
    enable_notifications(&app, on)
}

/// Se ligar falhar, a opção fica desligada: não adianta tentar de novo a cada
/// abertura, e o erro volta para a tela explicar o motivo.
fn enable_notifications(app: &AppHandle, on: bool) -> Result<(), String> {
    set_notifications(app, on).inspect_err(|_| {
        let _ = set_notifications(app, false);
    })
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
fn save_shortcuts(
    state: State<'_, AppState>,
    shortcuts: Vec<store::Shortcut>,
) -> Result<store::Data, String> {
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

/// A janela é criada aqui, e não no tauri.conf.json, porque o tamanho depende
/// do notch detectado.
fn create_island(app: &AppHandle, layout: Layout) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, ISLAND, WebviewUrl::default())
        .title("Notchfy")
        .inner_size(layout.expanded_width, layout.expanded_height)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
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
                    let state = app.state::<AppState>();
                    // O menu da bandeja não tem onde mostrar erro; na pior das
                    // hipóteses a ilha não alterna e o usuário tenta de novo.
                    let _ = if w.is_visible().unwrap_or(false) {
                        *state.hover_area.lock().unwrap() = None;
                        w.hide()
                    } else {
                        show_collapsed(&w, &state)
                    };
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
        // Precisa ser o primeiro plugin. Abrir o app de novo (ou um hook antigo
        // que chame o executável sem argumentos) só mostra a ilha que já existe.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(w) = app.get_webview_window(ISLAND) {
                // Sem onde mostrar erro: no pior caso a ilha segue oculta.
                let _ = show_collapsed(&w, &app.state::<AppState>());
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_drag::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
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
                placement: Mutex::new(None),
                hover_area: hover::Area::default(),
                notices: Mutex::new(None),
            });

            if let Err(e) = build_tray(&handle) {
                eprintln!("notchfy: bandeja indisponível: {e}");
            }

            let window = create_island(&handle, layout)?;
            platform::configure_window(&window);
            let state = app.state::<AppState>();
            show_collapsed(&window, &state)?;
            hover::watch(window, state.hover_area.clone());
            battery::watch(move |alert| {
                // emit só falha com o app fechando; não há a quem avisar.
                let _ = handle.emit(battery::BATTERY_EVENT, alert);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_layout,
            set_expanded,
            notifications_init,
            set_notifications_enabled,
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
