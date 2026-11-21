//! Uso de tokens lido dos logs locais do Claude Code e do Codex CLI.
//!
//! Nenhum dado sai da máquina: os dois CLIs gravam cada sessão em JSONL e aqui
//! apenas somamos os campos de uso. O uso feito pelos sites (claude.ai,
//! chatgpt.com) não aparece nesses arquivos.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Local, NaiveDate};
use serde::Serialize;
use serde_json::Value;

const WINDOW_DAYS: i64 = 7;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Totals {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub total: u64,
}

impl Totals {
    fn add(&mut self, o: &Totals) {
        self.input += o.input;
        self.output += o.output;
        self.cache_read += o.cache_read;
        self.cache_write += o.cache_write;
        self.total += o.total;
    }
}

#[derive(Debug, Serialize)]
pub struct DayTotal {
    pub date: String,
    pub total: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderReport {
    pub id: &'static str,
    pub name: &'static str,
    /// Encontramos pelo menos uma pasta de logs.
    pub found: bool,
    pub dirs: Vec<String>,
    pub today: Totals,
    pub week: Totals,
    pub days: Vec<DayTotal>,
    pub last_activity: Option<String>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub providers: Vec<ProviderReport>,
    pub generated_at: String,
}

#[derive(Clone, Debug)]
struct Entry {
    /// Chave para descartar a mesma resposta registrada em dois arquivos
    /// (o Claude Code copia mensagens ao retomar uma sessão).
    key: Option<String>,
    ts: DateTime<Local>,
    t: Totals,
}

struct FileCache {
    mtime: SystemTime,
    len: u64,
    entries: Vec<Entry>,
}

/// Evita reler arquivos que não mudaram desde a última consulta.
#[derive(Default)]
pub struct Cache {
    files: Mutex<HashMap<PathBuf, FileCache>>,
}

pub fn report(cache: &Cache) -> Report {
    let today = Local::now().date_naive();
    Report {
        providers: vec![
            build("claude", "Claude Code", &claude_dirs(), parse_claude, cache, today),
            build("codex", "Codex", &codex_dirs(), parse_codex, cache, today),
        ],
        generated_at: Local::now().to_rfc3339(),
    }
}

fn existing(dirs: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    for d in dirs {
        if d.is_dir() && !out.contains(&d) {
            out.push(d);
        }
    }
    out
}

fn claude_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(v) = std::env::var("CLAUDE_CONFIG_DIR") {
        dirs.extend(v.split(',').map(|p| Path::new(p.trim()).join("projects")));
    }
    if let Some(h) = dirs::home_dir() {
        dirs.push(h.join(".config/claude/projects"));
        dirs.push(h.join(".claude/projects"));
    }
    existing(dirs)
}

fn codex_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".codex")));
    existing(home.into_iter().flat_map(|h| [h.join("sessions"), h.join("archived_sessions")]))
}

fn collect_jsonl(dir: &Path, cutoff: SystemTime, out: &mut Vec<PathBuf>, depth: u8) {
    if depth > 6 {
        return;
    }
    let Ok(read) = fs::read_dir(dir) else { return };
    for entry in read.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            collect_jsonl(&path, cutoff, out, depth + 1);
        } else if path.extension().is_some_and(|e| e == "jsonl")
            && meta.modified().is_ok_and(|m| m >= cutoff)
        {
            out.push(path);
        }
    }
}

fn build(
    id: &'static str,
    name: &'static str,
    dirs: &[PathBuf],
    parse: fn(&str) -> Vec<Entry>,
    cache: &Cache,
    today: NaiveDate,
) -> ProviderReport {
    let start = today - chrono::Duration::days(WINDOW_DAYS - 1);
    // Um dia a mais de folga: um arquivo pode ter sido tocado por último logo
    // antes da meia-noite do primeiro dia da janela.
    let cutoff = SystemTime::now() - Duration::from_secs(86_400 * (WINDOW_DAYS as u64 + 1));
    let mut files = Vec::new();
    for d in dirs {
        collect_jsonl(d, cutoff, &mut files, 0);
    }

    let mut by_day: HashMap<NaiveDate, Totals> = HashMap::new();
    let mut seen = HashSet::new();
    let mut last: Option<DateTime<Local>> = None;
    let mut guard = cache.files.lock().unwrap();
    guard.retain(|p, _| files.contains(p));

    for file in &files {
        let Ok(meta) = fs::metadata(file) else { continue };
        let (mtime, len) = (meta.modified().unwrap_or(SystemTime::UNIX_EPOCH), meta.len());
        let fresh = guard.get(file).is_some_and(|c| c.mtime == mtime && c.len == len);
        if !fresh {
            let Ok(bytes) = fs::read(file) else { continue };
            let entries = parse(&String::from_utf8_lossy(&bytes));
            guard.insert(file.clone(), FileCache { mtime, len, entries });
        }
        for e in &guard[file].entries {
            if let Some(k) = &e.key {
                if !seen.insert(k.clone()) {
                    continue;
                }
            }
            let day = e.ts.date_naive();
            if day < start || day > today {
                continue;
            }
            by_day.entry(day).or_default().add(&e.t);
            if last.is_none_or(|l| e.ts > l) {
                last = Some(e.ts);
            }
        }
    }

    let mut week = Totals::default();
    let days = (0..WINDOW_DAYS)
        .map(|i| {
            let date = start + chrono::Duration::days(i);
            let t = by_day.get(&date).copied().unwrap_or_default();
            week.add(&t);
            DayTotal { date: date.to_string(), total: t.total }
        })
        .collect();

    ProviderReport {
        id,
        name,
        found: !dirs.is_empty(),
        dirs: dirs.iter().map(|d| d.to_string_lossy().into_owned()).collect(),
        today: by_day.get(&today).copied().unwrap_or_default(),
        week,
        days,
        last_activity: last.map(|t| t.to_rfc3339()),
    }
}

fn timestamp(v: &Value) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(v["timestamp"].as_str()?)
        .ok()
        .map(|t| t.with_timezone(&Local))
}

fn num(v: &Value) -> u64 {
    v.as_u64().unwrap_or(0)
}

/// Linhas do Claude Code: `{"type":"assistant","timestamp":…,"requestId":…,
/// "message":{"id":…,"usage":{"input_tokens",…}}}`.
fn parse_claude(content: &str) -> Vec<Entry> {
    content
        .lines()
        .filter(|l| l.contains("\"usage\""))
        .filter_map(|line| {
            let v: Value = serde_json::from_str(line).ok()?;
            let u = &v["message"]["usage"];
            if !u.is_object() {
                return None;
            }
            let mut t = Totals {
                input: num(&u["input_tokens"]),
                output: num(&u["output_tokens"]),
                cache_read: num(&u["cache_read_input_tokens"]),
                cache_write: num(&u["cache_creation_input_tokens"]),
                total: 0,
            };
            t.total = t.input + t.output + t.cache_read + t.cache_write;
            if t.total == 0 {
                return None;
            }
            let key = v["message"]["id"].as_str().map(|id| match v["requestId"].as_str() {
                Some(req) => format!("{id}:{req}"),
                None => id.to_string(),
            });
            Some(Entry { key, ts: timestamp(&v)?, t })
        })
        .collect()
}

/// Linhas do Codex: eventos `token_count` com o total acumulado da sessão.
/// O consumo de cada momento é a diferença entre um total e o anterior, o que
/// também ignora eventos repetidos.
fn parse_codex(content: &str) -> Vec<Entry> {
    #[derive(Clone, Copy, Default)]
    struct Cum {
        input: u64,
        cached: u64,
        output: u64,
        total: u64,
    }
    let mut prev = Cum::default();
    let mut out = Vec::new();
    for line in content.lines().filter(|l| l.contains("token_count")) {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let payload = if v["payload"].is_object() { &v["payload"] } else { &v };
        if payload["type"] != "token_count" {
            continue;
        }
        let u = &payload["info"]["total_token_usage"];
        if !u.is_object() {
            continue;
        }
        let cur = Cum {
            input: num(&u["input_tokens"]),
            cached: num(&u["cached_input_tokens"]),
            output: num(&u["output_tokens"]),
            total: num(&u["total_tokens"]),
        };
        // Total menor que o anterior: o contador recomeçou.
        let base = if cur.total < prev.total { Cum::default() } else { prev };
        prev = cur;
        let d_input = cur.input.saturating_sub(base.input);
        let d_cached = cur.cached.saturating_sub(base.cached);
        let t = Totals {
            input: d_input.saturating_sub(d_cached),
            output: cur.output.saturating_sub(base.output),
            cache_read: d_cached,
            cache_write: 0,
            total: cur.total.saturating_sub(base.total),
        };
        if t.total == 0 {
            continue;
        }
        if let Some(ts) = timestamp(&v) {
            out.push(Entry { key: None, ts, t });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_lines_are_summed_and_keyed() {
        let log = concat!(
            r#"{"type":"user","timestamp":"2026-11-20T10:00:00Z","message":{"role":"user"}}"#, "\n",
            r#"{"type":"assistant","timestamp":"2026-11-20T10:00:05Z","requestId":"r1","message":{"id":"m1","usage":{"input_tokens":10,"output_tokens":20,"cache_read_input_tokens":300,"cache_creation_input_tokens":40}}}"#, "\n",
            "isto não é json \"usage\"\n",
        );
        let entries = parse_claude(log);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key.as_deref(), Some("m1:r1"));
        assert_eq!(entries[0].t, Totals { input: 10, output: 20, cache_read: 300, cache_write: 40, total: 370 });
    }

    #[test]
    fn codex_uses_deltas_and_skips_repeats() {
        let ev = |ts: &str, input: u64, cached: u64, output: u64| {
            format!(
                r#"{{"timestamp":"{ts}","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":{input},"cached_input_tokens":{cached},"output_tokens":{output},"total_tokens":{}}}}}}}}}"#,
                input + output
            )
        };
        let log = [
            ev("2026-11-20T10:00:00Z", 100, 0, 10),
            ev("2026-11-20T10:00:01Z", 100, 0, 10), // repetido
            ev("2026-11-20T10:05:00Z", 250, 80, 30),
            r#"{"timestamp":"2026-11-20T10:06:00Z","type":"event_msg","payload":{"type":"token_count","info":null}}"#.to_string(),
        ]
        .join("\n");
        let entries = parse_codex(&log);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].t, Totals { input: 100, output: 10, cache_read: 0, cache_write: 0, total: 110 });
        assert_eq!(entries[1].t, Totals { input: 70, output: 20, cache_read: 80, cache_write: 0, total: 170 });
    }

    #[test]
    fn duplicated_claude_messages_count_once() {
        let dir = std::env::temp_dir().join(format!("notchfy-tokens-{}", std::process::id()));
        fs::create_dir_all(dir.join("proj")).unwrap();
        let now = Local::now().to_rfc3339();
        let line = format!(
            r#"{{"type":"assistant","timestamp":"{now}","requestId":"r","message":{{"id":"m","usage":{{"input_tokens":5,"output_tokens":5}}}}}}"#
        );
        fs::write(dir.join("proj/a.jsonl"), &line).unwrap();
        fs::write(dir.join("proj/b.jsonl"), &line).unwrap();
        let cache = Cache::default();
        let r = build("claude", "Claude", &[dir.clone()], parse_claude, &cache, Local::now().date_naive());
        assert_eq!(r.today.total, 10);
        assert_eq!(r.week.total, 10);
        assert_eq!(r.days.len(), WINDOW_DAYS as usize);
        // Segunda leitura vem do cache e precisa dar o mesmo resultado.
        let again = build("claude", "Claude", &[dir.clone()], parse_claude, &cache, Local::now().date_naive());
        assert_eq!(again.today.total, 10);
        let _ = fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod local {
    /// `cargo test -- --ignored --nocapture print_local_report` mostra o
    /// relatório real desta máquina.
    #[test]
    #[ignore]
    fn print_local_report() {
        let r = super::report(&super::Cache::default());
        println!("{}", serde_json::to_string_pretty(&r).unwrap());
    }
}
