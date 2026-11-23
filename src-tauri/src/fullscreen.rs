//! Esconde a ilha enquanto um vídeo, um jogo ou uma apresentação ocupa a tela,
//! e mostra de novo quando sai.
//!
//! - Windows: `SHQueryUserNotificationState`, o mesmo sinal que o Windows usa
//!   para segurar as próprias notificações. Não distingue monitores: tela
//!   cheia no segundo monitor também esconde a ilha.
//! - macOS: resolvido em `platform.rs`; a janela não entra nos espaços de tela cheia.
//! - Linux: não detectado. O app roda pelo XWayland, que não enxerga janelas
//!   Wayland em tela cheia, e vários gerenciadores já põem a janela em tela
//!   cheia por cima das "sempre no topo".

#[derive(Debug, PartialEq)]
pub enum Action {
    Hide,
    Show,
}

/// Só desfaz o que ele mesmo fez: se a pessoa ocultou a ilha pela bandeja,
/// sair da tela cheia não a traz de volta.
#[derive(Default)]
pub struct Tracker {
    hidden_by_us: bool,
}

impl Tracker {
    pub fn next(&mut self, fullscreen: bool, visible: bool) -> Option<Action> {
        if fullscreen && visible {
            self.hidden_by_us = true;
            return Some(Action::Hide);
        }
        if !fullscreen && std::mem::take(&mut self.hidden_by_us) && !visible {
            return Some(Action::Show);
        }
        None
    }
}

#[cfg(windows)]
pub fn is_fullscreen() -> bool {
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE,
        QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    // SAFETY: a função não recebe nada nosso; só devolve o estado atual.
    match unsafe { SHQueryUserNotificationState() } {
        Ok(state) => [
            QUNS_BUSY,
            QUNS_RUNNING_D3D_FULL_SCREEN,
            QUNS_PRESENTATION_MODE,
        ]
        .contains(&state),
        // Sem resposta do sistema, a ilha fica como está.
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_in_fullscreen_and_comes_back() {
        let mut t = Tracker::default();
        assert_eq!(t.next(false, true), None);
        assert_eq!(t.next(true, true), Some(Action::Hide));
        assert_eq!(t.next(true, false), None);
        assert_eq!(t.next(false, false), Some(Action::Show));
        assert_eq!(t.next(false, true), None);
    }

    #[test]
    fn respects_a_hide_from_the_tray() {
        let mut t = Tracker::default();
        // Ocultada pela pessoa, sem tela cheia: nada a fazer em nenhum momento.
        assert_eq!(t.next(false, false), None);
        assert_eq!(t.next(true, false), None);
        assert_eq!(t.next(false, false), None);
    }
}
