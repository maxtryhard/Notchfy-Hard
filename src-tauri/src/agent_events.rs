//! Avisos do Claude Code e do Codex na ilha ("terminou", "precisa de você").
//!
//! Os dois CLIs chamam o próprio executável do Notchfy pelos hooks oficiais
//! (`notchfy --hook claude` / `notchfy --hook codex`, configurados em
//! `agent_setup`). Esse processo curto não abre janela: lê o evento, grava um
//! arquivo pequeno numa pasta do app e sai. O app aberto lê essa pasta a cada
//! segundo e mostra o aviso. Nada sai da máquina e nenhuma credencial é lida.

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const AGENT_EVENT: &str = "island-agent";

/// Mesmo identificador do tauri.conf.json: a pasta precisa ser a mesma no
/// processo do hook (que não inicia o Tauri) e no app.
const APP_ID: &str = "app.notchfy.hard";
const POLL: Duration = Duration::from_secs(1);
/// Eventos mais velhos que isso são descartados sem aparecer: abrir o app
/// não deve despejar avisos de horas atrás.
const MAX_AGE: Duration = Duration::from_secs(120);
const MAX_PAYLOAD: u64 = 256 * 1024;
const MAX_FILE: u64 = 16 * 1024;
const MAX_FILES_PER_POLL: usize = 20;
const MAX_MESSAGE_CHARS: usize = 300;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Source {
    Claude,
    Codex,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    /// Terminou de responder.
    Done,
    /// Parou esperando a pessoa (permissão, pergunta, ociosidade).
    NeedsYou,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentNotice {
    pub source: Source,
    pub kind: Kind,
    /// Nome da pasta em que o agente trabalha.
    pub project: String,
    pub message: String,
}

/// Tipos de `Notification` do Claude Code que significam "parei, preciso de
/// você". Os demais (login, cota…) não interessam à ilha.
pub const CLAUDE_NEEDS_YOU: [&str; 5] = [
    "permission_prompt",
    "idle_prompt",
    "elicitation_dialog",
    "elicitation_url_dialog",
    "agent_needs_input",
];

fn project_name(cwd: Option<&str>) -> String {
    cwd.and_then(|c| Path::new(c).file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Uma linha só e no máximo `MAX_MESSAGE_CHARS`: a ilha mostra duas linhas.
fn one_line(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    match flat.char_indices().nth(MAX_MESSAGE_CHARS) {
        Some((cut, _)) => format!("{}…", &flat[..cut]),
        None => flat,
    }
}

/// JSON que o Claude Code manda pela entrada padrão do hook.
pub fn parse_claude(payload: &str) -> Option<AgentNotice> {
    let v: Value = serde_json::from_str(payload).ok()?;
    let text = |key: &str| v.get(key).and_then(Value::as_str);
    let (kind, message) = match text("hook_event_name")? {
        "Stop" => (Kind::Done, text("last_assistant_message").unwrap_or("")),
        "Notification" if CLAUDE_NEEDS_YOU.contains(&text("notification_type")?) => {
            (Kind::NeedsYou, text("message").unwrap_or(""))
        }
        _ => return None,
    };
    Some(AgentNotice {
        source: Source::Claude,
        kind,
        project: project_name(text("cwd")),
        message: one_line(message),
    })
}

/// JSON que o Codex passa como último argumento do programa de `notify`.
pub fn parse_codex(payload: &str) -> Option<AgentNotice> {
    let v: Value = serde_json::from_str(payload).ok()?;
    let text = |key: &str| v.get(key).and_then(Value::as_str);
    if text("type")? != "agent-turn-complete" {
        return None;
    }
    Some(AgentNotice {
        source: Source::Codex,
        kind: Kind::Done,
        project: project_name(text("cwd")),
        message: one_line(text("last-assistant-message").unwrap_or("")),
    })
}

fn spool_dir() -> Option<PathBuf> {
    Some(dirs::data_dir()?.join(APP_ID).join("agent-events"))
}

/// Modo hook: se o processo foi aberto como `notchfy --hook <fonte>`, trata o
/// evento e devolve `true` para o app sair sem abrir janela. Sai em silêncio
/// mesmo com erro: o hook não pode atrapalhar o Claude Code nem o Codex.
pub fn handle_hook_invocation() -> bool {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("--hook") {
        return false;
    }
    let notice = match args.get(1).map(String::as_str) {
        Some("claude") => {
            let mut payload = String::new();
            // Entrada que não é texto ou passa do limite: evento ignorado.
            let _ = std::io::stdin()
                .take(MAX_PAYLOAD)
                .read_to_string(&mut payload);
            parse_claude(&payload)
        }
        Some("codex") => {
            let payload = args.last().filter(|a| a.len() as u64 <= MAX_PAYLOAD);
            if let Some(payload) = payload {
                forward_to_previous_notify(&args, payload);
            }
            payload.and_then(|a| parse_codex(a))
        }
        _ => None,
    };
    if let (Some(notice), Some(dir)) = (notice, spool_dir()) {
        // Sem como avisar ninguém daqui; o evento só não aparece.
        let _ = write_event(&dir, &notice);
    }
    true
}

/// Programa de `notify` que o Codex já tinha antes do Notchfy, guardado
/// depois de `--` (`notchfy --hook codex -- <programa> <args…> <json>`).
fn previous_notify(args: &[String]) -> Option<&[String]> {
    let rest = args.get(2..args.len().checked_sub(1)?)?;
    rest.split_first()
        .filter(|(first, cmd)| first.as_str() == "--" && !cmd.is_empty())
        .map(|(_, cmd)| cmd)
}

/// Repassa o aviso para o programa anterior, como o Codex faria, para ele
/// continuar funcionando. Roda o que a própria pessoa (ou o app do Codex)
/// configurou, com a lista de argumentos original e sem shell.
fn forward_to_previous_notify(args: &[String], payload: &str) {
    let Some((program, rest)) = previous_notify(args).and_then(<[String]>::split_first) else {
        return;
    };
    let mut command = std::process::Command::new(program);
    command.args(rest).arg(payload);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Sem janela de console piscando: o Notchfy roda sem console.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    // Se o programa anterior sumiu (ex.: o Codex foi atualizado), só ele deixa
    // de rodar; o aviso na ilha continua.
    let _ = command.spawn();
}

/// Grava num arquivo temporário e renomeia, para o app nunca ler pela metade.
fn write_event(dir: &Path, notice: &AgentNotice) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = format!("{nanos}-{}", std::process::id());
    let tmp = dir.join(format!("{name}.tmp"));
    fs::write(&tmp, serde_json::to_vec(notice)?)?;
    fs::rename(tmp, dir.join(format!("{name}.json")))
}

/// Lê e apaga os eventos pendentes, do mais antigo para o mais novo.
fn take_events(dir: &Path, now: SystemTime) -> Vec<AgentNotice> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new(); // Pasta ainda não existe: nenhum hook rodou.
    };
    let mut files: Vec<(SystemTime, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let meta = e.metadata().ok()?;
            let ext = path.extension()?.to_str()?;
            (meta.is_file() && (ext == "json" || ext == "tmp"))
                .then(|| (meta.modified().unwrap_or(now), path))
        })
        .collect();
    files.sort();
    let mut out = Vec::new();
    for (modified, path) in files.into_iter().take(MAX_FILES_PER_POLL) {
        let stale = now.duration_since(modified).unwrap_or_default() > MAX_AGE;
        let is_json = path.extension().is_some_and(|e| e == "json");
        if !is_json && !stale {
            continue; // .tmp de um hook ainda gravando.
        }
        let small = fs::metadata(&path).is_ok_and(|m| m.len() <= MAX_FILE);
        if is_json && small && !stale {
            if let Some(n) = fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
            {
                out.push(n);
            }
        }
        // Se não der para apagar, o evento envelhece e é descartado depois.
        let _ = fs::remove_file(&path);
    }
    out
}

/// Fica de olho na pasta de eventos e chama `on_notice` para cada um.
pub fn watch(on_notice: impl Fn(AgentNotice) + Send + 'static) {
    let Some(dir) = spool_dir() else { return };
    thread::spawn(move || loop {
        for notice in take_events(&dir, SystemTime::now()) {
            on_notice(notice);
        }
        thread::sleep(POLL);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // Exemplos da documentação oficial dos hooks do Claude Code.
    const CLAUDE_STOP: &str = r#"{
      "session_id": "abc123",
      "transcript_path": "~/.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
      "cwd": "/Users/ana/projetos/loja",
      "permission_mode": "default",
      "hook_event_name": "Stop",
      "stop_hook_active": true,
      "last_assistant_message": "I've completed the refactoring.\n\nHere's a summary...",
      "background_tasks": [],
      "session_crons": []
    }"#;
    const CLAUDE_PERMISSION: &str = r#"{
      "session_id": "abc123",
      "transcript_path": "/Users/.../.claude/projects/.../00893aaf-19fa-41d2-8238-13269b9b3ca0.jsonl",
      "cwd": "/Users/ana/projetos/loja",
      "hook_event_name": "Notification",
      "message": "Claude needs your permission",
      "notification_type": "permission_prompt"
    }"#;
    // Campos listados na documentação do `notify` do Codex.
    const CODEX_TURN: &str = r#"{
      "type": "agent-turn-complete",
      "thread-id": "b5f6c1c2-1111-2222-3333-444455556666",
      "turn-id": "12345",
      "cwd": "/home/ana/api",
      "input-messages": ["Rename `foo` to `bar` and update the callsites."],
      "last-assistant-message": "Rename complete and verified `cargo build` succeeds."
    }"#;

    #[test]
    fn reads_claude_stop() {
        let n = parse_claude(CLAUDE_STOP).unwrap();
        assert_eq!(n.kind, Kind::Done);
        assert_eq!(n.project, "loja");
        assert_eq!(
            n.message,
            "I've completed the refactoring. Here's a summary..."
        );
    }

    #[test]
    fn reads_claude_permission_prompt() {
        let n = parse_claude(CLAUDE_PERMISSION).unwrap();
        assert_eq!(n.kind, Kind::NeedsYou);
        assert_eq!(n.message, "Claude needs your permission");
    }

    #[test]
    fn ignores_other_claude_events() {
        let login = CLAUDE_PERMISSION.replace("permission_prompt", "auth_success");
        assert_eq!(parse_claude(&login), None);
        let tool = CLAUDE_STOP.replace("\"Stop\"", "\"PreToolUse\"");
        assert_eq!(parse_claude(&tool), None);
        assert_eq!(parse_claude("não é json"), None);
    }

    #[test]
    fn reads_codex_turn() {
        let n = parse_codex(CODEX_TURN).unwrap();
        assert_eq!((n.source, n.kind), (Source::Codex, Kind::Done));
        assert_eq!(n.project, "api");
        assert_eq!(
            n.message,
            "Rename complete and verified `cargo build` succeeds."
        );
        assert_eq!(parse_codex(r#"{"type":"other"}"#), None);
    }

    #[test]
    fn finds_the_chained_notify() {
        let args = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let chained = args(&[
            "--hook",
            "codex",
            "--",
            "computer-use.exe",
            "turn-ended",
            "{}",
        ]);
        assert_eq!(
            previous_notify(&chained).unwrap(),
            ["computer-use.exe", "turn-ended"]
        );
        assert_eq!(previous_notify(&args(&["--hook", "codex", "{}"])), None);
        assert_eq!(
            previous_notify(&args(&["--hook", "codex", "--", "{}"])),
            None
        );
    }

    #[test]
    fn shortens_long_messages() {
        let long = "palavra ".repeat(100);
        let short = one_line(&long);
        assert_eq!(short.chars().count(), MAX_MESSAGE_CHARS + 1);
        assert!(short.ends_with('…'));
    }

    #[test]
    fn spool_round_trip_and_staleness() {
        let dir = std::env::temp_dir().join(format!("notchfy-spool-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let notice = parse_codex(CODEX_TURN).unwrap();
        write_event(&dir, &notice).unwrap();
        write_event(&dir, &notice).unwrap();
        assert_eq!(
            take_events(&dir, SystemTime::now()),
            vec![notice.clone(), notice.clone()]
        );
        // Lidos são apagados.
        assert!(take_events(&dir, SystemTime::now()).is_empty());
        // Eventos velhos somem sem aparecer.
        write_event(&dir, &notice).unwrap();
        let later = SystemTime::now() + MAX_AGE + Duration::from_secs(1);
        assert!(take_events(&dir, later).is_empty());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        fs::remove_dir_all(&dir).unwrap();
    }
}
