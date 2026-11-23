//! Liga e desliga os hooks que mandam os avisos do Claude Code e do Codex
//! para a ilha (ver `agent_events`). Só roda quando a pessoa clica no botão.
//!
//! - Claude Code: entradas `Stop` e `Notification` em `~/.claude/settings.json`.
//! - Codex: a chave `notify` em `~/.codex/config.toml`.
//!
//! Só mexemos nas entradas que apontam para o executável do Notchfy. O resto
//! do arquivo fica como estava, e o conteúdo nunca é mostrado nem registrado:
//! o settings.json pode ter tokens no bloco `env`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::agent_events::{Source, CLAUDE_NEEDS_YOU};

const MAX_CONFIG: u64 = 1024 * 1024;

#[derive(Serialize)]
pub struct Status {
    claude: bool,
    codex: bool,
}

pub fn status() -> Status {
    Status {
        claude: claude_settings()
            .and_then(|p| read_config(&p).ok().flatten())
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .is_some_and(|v| claude_has_ours(&v)),
        codex: codex_config()
            .and_then(|p| read_config(&p).ok().flatten())
            .is_some_and(|t| codex_notify_line(&t).is_some_and(|(_, ours)| ours)),
    }
}

pub fn set(source: Source, on: bool) -> Result<(), String> {
    match source {
        Source::Claude => {
            let path = claude_settings().ok_or("Pasta pessoal não encontrada.")?;
            let text = read_config(&path)?.unwrap_or_else(|| "{}".into());
            let mut v: Value = serde_json::from_str(&text).map_err(|_| {
                format!(
                    "{} não é um JSON válido; corrija antes de ligar.",
                    path.display()
                )
            })?;
            let exe = if on { Some(hook_executable()?) } else { None };
            claude_apply(&mut v, exe.as_deref())?;
            let mut out = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
            out.push('\n');
            write_config(&path, &out)
        }
        Source::Codex => {
            let path = codex_config().ok_or("Pasta pessoal não encontrada.")?;
            let text = read_config(&path)?.unwrap_or_default();
            let exe = if on { Some(hook_executable()?) } else { None };
            write_config(&path, &codex_apply(&text, exe.as_deref())?)
        }
    }
}

fn claude_settings() -> Option<PathBuf> {
    let dir = match std::env::var("CLAUDE_CONFIG_DIR") {
        Ok(v) if !v.trim().is_empty() => PathBuf::from(v.split(',').next()?.trim()),
        _ => dirs::home_dir()?.join(".claude"),
    };
    Some(dir.join("settings.json"))
}

fn codex_config() -> Option<PathBuf> {
    let home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".codex")))?;
    Some(home.join("config.toml"))
}

/// `None` quando o arquivo ainda não existe.
fn read_config(path: &Path) -> Result<Option<String>, String> {
    match fs::metadata(path) {
        Err(_) => return Ok(None),
        Ok(m) if m.len() > MAX_CONFIG => {
            return Err(format!("{} é grande demais para editar.", path.display()))
        }
        Ok(_) => {}
    }
    fs::read_to_string(path)
        .map(Some)
        .map_err(|e| format!("Não foi possível ler {}: {e}", path.display()))
}

/// Grava ao lado e renomeia: se algo falhar no meio, o original fica intacto.
fn write_config(path: &Path, text: &str) -> Result<(), String> {
    let fail = |e: std::io::Error| format!("Não foi possível gravar {}: {e}", path.display());
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(fail)?;
    }
    let tmp = path.with_extension("notchfy-tmp");
    fs::write(&tmp, text).map_err(fail)?;
    fs::rename(&tmp, path).map_err(fail)
}

/// Caminho que os hooks vão chamar. Precisa continuar existindo depois que o
/// app fecha, por isso o AppImage usa o próprio arquivo, não a montagem.
fn hook_executable() -> Result<String, String> {
    let exe = std::env::var("APPIMAGE")
        .ok()
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .map_or_else(std::env::current_exe, Ok)
        .map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy().into_owned();
    if exe.contains("/AppTranslocation/") {
        return Err(
            "Mova o Notchfy para a pasta Aplicativos e abra de novo antes de ligar.".into(),
        );
    }
    Ok(exe)
}

fn hook_args(source: &str) -> [String; 2] {
    ["--hook".into(), source.into()]
}

/// Um hook é nosso quando chama `--hook claude` num executável do Notchfy,
/// esteja ele onde estiver (o caminho muda se o app for reinstalado).
fn is_ours(hook: &Value) -> bool {
    let args_match = hook.get("args").and_then(Value::as_array).is_some_and(|a| {
        a.iter()
            .map(Value::as_str)
            .eq(hook_args("claude").iter().map(|s| Some(s.as_str())))
    });
    // Separa nas duas barras à mão: o caminho pode ter vindo de outro sistema.
    let exe_match = hook
        .get("command")
        .and_then(Value::as_str)
        .and_then(|c| c.rsplit(['/', '\\']).next())
        .is_some_and(|name| name.to_lowercase().starts_with("notchfy"));
    args_match && exe_match
}

fn claude_has_ours(v: &Value) -> bool {
    v.pointer("/hooks/Stop")
        .and_then(Value::as_array)
        .is_some_and(|groups| {
            groups.iter().any(|g| {
                g.get("hooks")
                    .and_then(Value::as_array)
                    .is_some_and(|hs| hs.iter().any(is_ours))
            })
        })
}

/// Tira os nossos hooks de todos os eventos e, se `exe` vier, põe de novo
/// apontando para ele. Grupos que ficam vazios são removidos.
fn claude_apply(v: &mut Value, exe: Option<&str>) -> Result<(), String> {
    let root = v
        .as_object_mut()
        .ok_or("O settings.json do Claude Code não é um objeto JSON.")?;
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or("O campo \"hooks\" do settings.json não é um objeto.")?;
    for groups in hooks.values_mut().filter_map(Value::as_array_mut) {
        for group in groups.iter_mut() {
            if let Some(hs) = group.get_mut("hooks").and_then(Value::as_array_mut) {
                hs.retain(|h| !is_ours(h));
            }
        }
        groups.retain(|g| {
            g.get("hooks")
                .and_then(Value::as_array)
                .is_none_or(|hs| !hs.is_empty())
        });
    }
    if let Some(exe) = exe {
        // Forma exec (`args`): sem shell, então espaços no caminho não quebram.
        // `async`: o Claude nunca espera a ilha.
        let hook = json!({
            "type": "command",
            "command": exe,
            "args": hook_args("claude"),
            "async": true,
            "timeout": 10
        });
        let entries = [
            ("Stop", json!({ "hooks": [hook.clone()] })),
            (
                "Notification",
                json!({ "matcher": CLAUDE_NEEDS_YOU.join("|"), "hooks": [hook] }),
            ),
        ];
        for (event, group) in entries {
            let list = hooks
                .entry(event)
                .or_insert_with(|| Value::Array(Vec::new()))
                .as_array_mut()
                .ok_or(format!("O campo hooks.{event} não é uma lista."))?;
            list.push(group);
        }
    }
    hooks.retain(|_, groups| groups.as_array().is_none_or(|g| !g.is_empty()));
    if hooks.is_empty() {
        root.remove("hooks");
    }
    Ok(())
}

/// Linha da chave `notify` no topo do config.toml (antes da primeira
/// `[tabela]`, onde chaves de topo precisam ficar) e se ela é nossa.
fn codex_notify_line(text: &str) -> Option<(usize, bool)> {
    text.lines()
        .take_while(|l| !l.trim_start().starts_with('['))
        .position(|l| {
            l.trim_start()
                .strip_prefix("notify")
                .is_some_and(|rest| rest.trim_start().starts_with('='))
        })
        .map(|i| {
            let line = text.lines().nth(i).unwrap_or("");
            (i, line.contains("\"--hook\"") && line.contains("\"codex\""))
        })
}

fn toml_string(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn codex_apply(text: &str, exe: Option<&str>) -> Result<String, String> {
    let mut lines: Vec<&str> = text.lines().collect();
    match codex_notify_line(text) {
        Some((_, false)) => {
            return Err(
                "O Codex já tem um \"notify\" configurado por outro programa; remova-o do config.toml para usar a ilha."
                    .into(),
            )
        }
        Some((i, true)) => {
            lines.remove(i);
        }
        None => {}
    }
    let new_line = exe.map(|exe| {
        let args = hook_args("codex").map(|a| toml_string(&a)).join(", ");
        format!("notify = [{}, {args}]", toml_string(exe))
    });
    let mut out: Vec<&str> = new_line.iter().map(String::as_str).collect();
    out.extend(lines);
    let mut joined = out.join("\n");
    if !joined.is_empty() {
        joined.push('\n');
    }
    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = r"C:\Program Files\Notchfy\notchfy.exe";

    #[test]
    fn adds_and_removes_claude_hooks_keeping_the_rest() {
        let original = json!({
            "model": "opus",
            "env": { "X": "1" },
            "hooks": {
                "Stop": [{ "hooks": [{ "type": "command", "command": "say pronto" }] }],
                "PreToolUse": [{ "matcher": "Bash", "hooks": [{ "type": "command", "command": "log" }] }]
            }
        });
        let mut v = original.clone();
        claude_apply(&mut v, Some(EXE)).unwrap();
        assert!(claude_has_ours(&v));
        assert_eq!(
            v.pointer("/hooks/Stop").unwrap().as_array().unwrap().len(),
            2
        );
        assert_eq!(
            v.pointer("/hooks/Notification/0/hooks/0/command").unwrap(),
            EXE
        );
        // Ligar de novo não duplica.
        claude_apply(&mut v, Some(EXE)).unwrap();
        assert_eq!(
            v.pointer("/hooks/Stop").unwrap().as_array().unwrap().len(),
            2
        );
        claude_apply(&mut v, None).unwrap();
        assert_eq!(v, original);
    }

    #[test]
    fn removing_from_empty_settings_leaves_no_trace() {
        let mut v = json!({});
        claude_apply(&mut v, Some("/opt/Notchfy/notchfy")).unwrap();
        claude_apply(&mut v, None).unwrap();
        assert_eq!(v, json!({}));
    }

    #[test]
    fn keeps_key_order() {
        let mut v: Value = serde_json::from_str(r#"{"z": 1, "a": 2}"#).unwrap();
        claude_apply(&mut v, Some(EXE)).unwrap();
        let keys: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, ["z", "a", "hooks"]);
    }

    #[test]
    fn toggles_codex_notify_at_the_top() {
        let original = "model = \"gpt-5\"\n\n[tui]\nnotifications = true\n";
        let on = codex_apply(original, Some(EXE)).unwrap();
        assert_eq!(
            on.lines().next().unwrap(),
            r#"notify = ["C:\\Program Files\\Notchfy\\notchfy.exe", "--hook", "codex"]"#
        );
        assert_eq!(codex_notify_line(&on), Some((0, true)));
        assert_eq!(codex_apply(&on, None).unwrap(), original);
        assert_eq!(codex_apply("", None).unwrap(), "");
    }

    #[test]
    fn refuses_someone_elses_codex_notify() {
        let theirs = "notify = [\"python3\", \"/home/ana/notify.py\"]\n";
        assert!(codex_apply(theirs, Some(EXE)).is_err());
        // Uma chave `notify` dentro de uma tabela não é a de topo.
        let in_table = "[mcp_servers.x]\nnotify = true\n";
        assert_eq!(codex_notify_line(in_table), None);
    }
}
