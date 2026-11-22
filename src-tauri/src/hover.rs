//! Detecta o mouse sobre a ilha consultando a posição do cursor, em vez de
//! depender do hover do WebView: no macOS uma janela sem foco pode não receber
//! eventos de mouse, e a ilha nunca pega o foco.

use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use tauri::{Emitter, WebviewWindow};

pub const HOVER_EVENT: &str = "island-hover";

/// Tempo parado sobre a ilha antes de abrir. Evita abrir quando o mouse só
/// atravessa o topo da tela a caminho de outra coisa.
const INTENT: Duration = Duration::from_millis(120);
/// Perto do topo consultamos rápido; longe dele, devagar, para gastar menos.
const FAST: Duration = Duration::from_millis(40);
const SLOW: Duration = Duration::from_millis(100);
/// Distância, em pixels físicos, abaixo da ilha que ainda conta como "perto".
const NEAR_TOP: f64 = 300.0;

/// Retângulo da janela na tela, em pixels físicos.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Rect {
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
}

/// Onde a ilha está agora; `None` com ela escondida. Quem redimensiona a
/// janela atualiza este valor, e assim cada ciclo do detector só precisa
/// perguntar ao sistema onde está o cursor (perguntar o tamanho da janela
/// também acordaria a thread principal à toa).
pub type Area = Arc<Mutex<Option<Rect>>>;

/// Transforma amostras "dentro/fora" em eventos, aplicando a espera de intenção.
#[derive(Default)]
struct Intent {
    reported: bool,
    entered_at: Option<Instant>,
}

impl Intent {
    /// Devolve `Some(true)` ao confirmar a entrada e `Some(false)` ao sair.
    fn update(&mut self, inside: bool, now: Instant) -> Option<bool> {
        if !inside {
            self.entered_at = None;
            return std::mem::take(&mut self.reported).then_some(false);
        }
        if self.reported {
            return None;
        }
        let entered = *self.entered_at.get_or_insert(now);
        if now.duration_since(entered) >= INTENT {
            self.reported = true;
            return Some(true);
        }
        None
    }
}

#[derive(Default)]
struct Sample {
    inside: bool,
    near_top: bool,
}

fn sample(window: &WebviewWindow, area: &Area) -> Sample {
    let Some(rect) = *area.lock().unwrap() else {
        return Sample::default();
    };
    // Sem posição do cursor (raro, ex.: sessão bloqueada) tratamos como fora.
    let Ok(cursor) = window.cursor_position() else {
        return Sample::default();
    };
    Sample {
        inside: rect.contains(cursor.x, cursor.y),
        near_top: cursor.y < rect.bottom + NEAR_TOP,
    }
}

pub fn watch(window: WebviewWindow, area: Area) {
    thread::spawn(move || {
        let mut intent = Intent::default();
        loop {
            let s = sample(&window, &area);
            if let Some(hovering) = intent.update(s.inside, Instant::now()) {
                // Só falha com a janela sendo destruída; não há a quem avisar.
                let _ = window.emit(HOVER_EVENT, hovering);
            }
            thread::sleep(if s.near_top { FAST } else { SLOW });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(base: Instant, ms: u64) -> Instant {
        base + Duration::from_millis(ms)
    }

    #[test]
    fn opens_only_after_staying_for_the_intent_delay() {
        let t = Instant::now();
        let mut i = Intent::default();
        assert_eq!(i.update(true, at(t, 0)), None);
        assert_eq!(i.update(true, at(t, 80)), None);
        assert_eq!(i.update(true, at(t, 120)), Some(true));
        assert_eq!(i.update(true, at(t, 200)), None, "não repete o evento");
        assert_eq!(i.update(false, at(t, 240)), Some(false));
        assert_eq!(i.update(false, at(t, 280)), None, "não repete a saída");
    }

    #[test]
    fn rect_edges() {
        let r = Rect {
            left: 610.0,
            top: 0.0,
            right: 830.0,
            bottom: 32.0,
        };
        assert!(r.contains(610.0, 0.0));
        assert!(r.contains(829.9, 31.9));
        assert!(!r.contains(609.9, 10.0));
        assert!(!r.contains(830.0, 10.0), "a borda direita já é fora");
        assert!(!r.contains(700.0, 32.0), "a borda de baixo já é fora");
    }

    #[test]
    fn passing_through_never_opens() {
        let t = Instant::now();
        let mut i = Intent::default();
        assert_eq!(i.update(true, at(t, 0)), None);
        assert_eq!(
            i.update(false, at(t, 40)),
            None,
            "saiu antes de abrir: nada a avisar"
        );
        assert_eq!(i.update(true, at(t, 80)), None, "a espera recomeça");
        assert_eq!(i.update(true, at(t, 190)), None);
        assert_eq!(i.update(true, at(t, 200)), Some(true));
    }
}
