//! Integração com cada sistema: detecção do notch e nível da janela.

/// Dimensões do recorte físico da tela, em pontos lógicos.
#[derive(Clone, Copy, Debug)]
pub struct Notch {
    pub width: f64,
    pub height: f64,
}

/// Ajustes de ambiente que precisam acontecer antes de o GTK/WebView iniciar.
pub fn prepare_environment() {
    // No Wayland um app não pode escolher a posição da própria janela (o GNOME
    // não implementa wlr-layer-shell). Pelo XWayland a ilha consegue ficar
    // fixa no topo. Quem quiser forçar outro backend define GDK_BACKEND.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("GDK_BACKEND").is_none() {
        std::env::set_var("GDK_BACKEND", "x11");
    }
}

#[cfg(target_os = "macos")]
pub fn detect_notch() -> Option<Notch> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSScreen;

    let mtm = MainThreadMarker::new()?;
    // O primeiro item é a tela com a barra de menus, a mesma em que a ilha é
    // posicionada.
    let screens = NSScreen::screens(mtm);
    let screen = screens.firstObject()?;
    let insets = screen.safeAreaInsets();
    if insets.top <= 0.0 {
        return None;
    }
    let frame = screen.frame();
    let left = screen.auxiliaryTopLeftArea();
    let right = screen.auxiliaryTopRightArea();
    let width = frame.size.width - left.size.width - right.size.width;
    (width > 0.0).then_some(Notch {
        width,
        height: insets.top,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn detect_notch() -> Option<Notch> {
    None
}

/// Coloca a janela acima da barra de menus e presente em todos os espaços.
#[cfg(target_os = "macos")]
pub fn configure_window(window: &tauri::WebviewWindow) {
    use objc2_app_kit::{NSMainMenuWindowLevel, NSWindow, NSWindowCollectionBehavior};

    let Ok(ptr) = window.ns_window() else { return };
    // SAFETY: o ponteiro vem do Tauri e aponta para a NSWindow viva desta
    // janela; só a acessamos na thread principal, durante o setup.
    unsafe {
        let ns_window: &NSWindow = &*(ptr as *const NSWindow);
        ns_window.setLevel(NSMainMenuWindowLevel + 3);
        ns_window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        ns_window.setHasShadow(false);
    }
}

// No Linux a janela nasce `resizable` (ver `create_island`): no GTK,
// `resize()` não tem efeito numa janela não redimensionável, que fica presa ao
// tamanho mínimo do conteúdo. Sem bordas, o usuário continua sem alças para
// redimensionar. Windows e macOS ficam com `resizable(false)` porque no Windows
// o Tauri cria bordas de redimensionar em janelas sem moldura.
#[cfg(not(target_os = "macos"))]
pub fn configure_window(_window: &tauri::WebviewWindow) {}
