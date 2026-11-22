//! Notificações de outros apps exibidas na ilha.
//!
//! Desligado por padrão: lido só depois que o usuário liga a opção na bandeja.
//! Nada é guardado; cada notificação vai direto para a tela.
//! - Linux: escuta o método `Notify` no D-Bus da sessão (o mesmo que os apps
//!   usam para pedir uma notificação).
//! - Windows: `UserNotificationListener`, a API oficial. A documentação exige
//!   que o app seja empacotado (MSIX, como na Microsoft Store); se o Windows
//!   recusar, o erro volta para a ilha mostrar ao usuário.
//! - macOS: não há API oficial; ainda não implementado.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use serde::Serialize;

pub const NOTICE_EVENT: &str = "island-notification";

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Notice {
    pub app: String,
    pub title: String,
    pub body: String,
}

/// Escuta enquanto existir; ao ser descartado, a thread de escuta termina.
pub struct Listener {
    stop: Arc<AtomicBool>,
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Começa a escutar. O erro explica por que não foi possível (ex.: o Windows
/// recusou o acesso) e é mostrado ao usuário. `on_problem` recebe falhas que
/// só aparecem depois de ligada, uma vez cada, para também chegarem à tela.
pub fn start(
    on_notice: impl Fn(Notice) + Send + 'static,
    on_problem: impl Fn(String) + Send + 'static,
) -> Result<Listener, String> {
    let stop = Arc::new(AtomicBool::new(false));
    imp::start(on_notice, on_problem, stop.clone())?;
    Ok(Listener { stop })
}

/// Tira marcações simples (`<b>`, `<i>`…) que alguns apps põem no texto. Um
/// `<` só conta como marcação se vier seguido de letra ou `/` e fechar com `>`;
/// assim "2 < 3" continua intacto.
#[cfg(any(target_os = "linux", test))]
fn plain(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let is_tag = after.starts_with(|c: char| c.is_ascii_alphabetic() || c == '/');
        match after.find('>').filter(|_| is_tag) {
            Some(close) => rest = &after[close + 1..],
            None => {
                out.push('<');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use dbus::{blocking::Connection, channel::MatchingReceiver, message::MatchRule, Message};
    use std::{thread, time::Duration};

    pub(super) fn parse(msg: &Message) -> Option<Notice> {
        // Assinatura de Notify: app, id a substituir, ícone, título, corpo, …
        let (app, _replaces, _icon, title, body): (String, u32, String, String, String) =
            msg.read5().ok()?;
        let notice = Notice {
            app,
            title: plain(&title),
            body: plain(&body),
        };
        (!notice.title.is_empty() || !notice.body.is_empty()).then_some(notice)
    }

    pub fn start(
        on_notice: impl Fn(Notice) + Send + 'static,
        on_problem: impl Fn(String) + Send + 'static,
        stop: Arc<AtomicBool>,
    ) -> Result<(), String> {
        let conn =
            Connection::new_session().map_err(|e| format!("Sem acesso ao D-Bus da sessão: {e}"))?;
        let rule = MatchRule::new_method_call()
            .with_interface("org.freedesktop.Notifications")
            .with_member("Notify");
        // Um "monitor" do D-Bus só observa: não responde nem altera a notificação,
        // que continua chegando ao daemon normal do sistema.
        conn.with_proxy(
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            Duration::from_secs(5),
        )
        .method_call::<(), _, _, _>(
            "org.freedesktop.DBus.Monitoring",
            "BecomeMonitor",
            (vec![rule.match_str()], 0u32),
        )
        .map_err(|e| format!("O D-Bus recusou observar as notificações: {e}"))?;
        conn.start_receive(
            rule,
            Box::new(move |msg, _| {
                if let Some(notice) = parse(&msg) {
                    on_notice(notice);
                }
                true
            }),
        );
        thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                if let Err(e) = conn.process(Duration::from_millis(500)) {
                    on_problem(format!("A conexão com o D-Bus caiu ({e})."));
                    break;
                }
            }
        });
        Ok(())
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::{collections::HashSet, thread, time::Duration};
    use windows::UI::Notifications::{
        KnownNotificationBindings,
        Management::{UserNotificationListener, UserNotificationListenerAccessStatus as Access},
        NotificationKinds, UserNotification,
    };

    /// Intervalo entre consultas à central de notificações.
    const POLL: Duration = Duration::from_millis(1500);

    fn read(n: &UserNotification) -> windows::core::Result<Notice> {
        let app = n.AppInfo()?.DisplayInfo()?.DisplayName()?.to_string();
        let binding = n
            .Notification()?
            .Visual()?
            .GetBinding(&KnownNotificationBindings::ToastGeneric()?)?;
        let texts: Vec<String> = binding
            .GetTextElements()?
            .into_iter()
            .filter_map(|t| t.Text().ok().map(|s| s.to_string()))
            .collect();
        Ok(Notice {
            app,
            title: texts.first().cloned().unwrap_or_default(),
            body: texts
                .get(1..)
                .map(|rest| rest.join("\n"))
                .unwrap_or_default(),
        })
    }

    pub fn start(
        on_notice: impl Fn(Notice) + Send + 'static,
        on_problem: impl Fn(String) + Send + 'static,
        stop: Arc<AtomicBool>,
    ) -> Result<(), String> {
        // O código do erro vem primeiro para nunca ser cortado na tela: com ele
        // dá para saber exatamente o motivo.
        let refused = |e: windows::core::Error| {
            format!(
                "Código {:#010x}: o Windows bloqueou este instalador. Deve funcionar pela versão da Microsoft Store.",
                e.code().0
            )
        };
        let listener = UserNotificationListener::Current().map_err(refused)?;
        let mut access = listener.GetAccessStatus().map_err(refused)?;
        if access == Access::Unspecified {
            access = listener
                .RequestAccessAsync()
                .and_then(|op| op.join())
                .map_err(refused)?;
        }
        if access != Access::Allowed {
            return Err(
                "Acesso negado. Libere em Configurações > Privacidade > Notificações.".into(),
            );
        }
        thread::spawn(move || {
            // As que já estavam na central não são novidade: só marca como vistas.
            let mut seen: Option<HashSet<u32>> = None;
            // Avisa uma vez por sequência de falhas, não a cada consulta.
            let mut failing = false;
            while !stop.load(Ordering::Relaxed) {
                let list = listener
                    .GetNotificationsAsync(NotificationKinds::Toast)
                    .and_then(|op| op.join());
                if let Err(e) = &list {
                    if !failing {
                        on_problem(format!(
                            "Código {:#010x}: o Windows não entregou as notificações a este instalador.",
                            e.code().0
                        ));
                    }
                    failing = true;
                }
                if let Ok(list) = list {
                    failing = false;
                    let current: Vec<UserNotification> = list.into_iter().collect();
                    let ids: HashSet<u32> = current.iter().filter_map(|n| n.Id().ok()).collect();
                    if let Some(known) = &seen {
                        for n in &current {
                            if n.Id().is_ok_and(|id| !known.contains(&id)) {
                                if let Ok(notice) = read(n) {
                                    on_notice(notice);
                                }
                            }
                        }
                    }
                    seen = Some(ids);
                }
                thread::sleep(POLL);
            }
        });
        Ok(())
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
mod imp {
    use super::*;

    pub fn start(
        _: impl Fn(Notice) + Send + 'static,
        _: impl Fn(String) + Send + 'static,
        _: Arc<AtomicBool>,
    ) -> Result<(), String> {
        Err("Notificações na ilha ainda não funcionam no macOS.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_markup() {
        assert_eq!(
            plain("<b>Maria</b>: chegou o <i>arquivo</i>"),
            "Maria: chegou o arquivo"
        );
        assert_eq!(plain("  2 < 3 e nada mais "), "2 < 3 e nada mais");
        assert_eq!(plain("a <3 b"), "a <3 b");
        assert_eq!(plain("sem marcação"), "sem marcação");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn reads_dbus_notify_call() {
        use dbus::{arg::PropMap, Message};
        let msg = Message::new_method_call(
            "org.freedesktop.Notifications",
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "Notify",
        )
        .unwrap()
        .append3("WhatsApp", 0u32, "")
        .append3("Maria", "<b>Oi!</b> Tudo bem?", Vec::<String>::new())
        .append2(PropMap::new(), 5000i32);
        assert_eq!(
            imp::parse(&msg),
            Some(Notice {
                app: "WhatsApp".into(),
                title: "Maria".into(),
                body: "Oi! Tudo bem?".into()
            })
        );
    }
}
