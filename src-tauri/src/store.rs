//! Dados persistidos: atalhos e itens da prateleira.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Shortcut {
    pub id: String,
    pub name: String,
    /// Emoji ou uma ou duas letras exibidas no botão.
    pub icon: String,
    /// `app`, `url`, `path` ou `command`.
    pub kind: String,
    pub target: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShelfItem {
    pub path: String,
    pub name: String,
    pub added_at: i64,
    #[serde(default)]
    pub is_dir: bool,
    /// Calculado a cada leitura; o arquivo pode ter sido movido ou apagado.
    #[serde(default, skip_deserializing)]
    pub exists: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Data {
    #[serde(default)]
    pub shortcuts: Vec<Shortcut>,
    #[serde(default)]
    pub shelf: Vec<ShelfItem>,
}

pub struct Store {
    path: PathBuf,
    pub data: Data,
}

const SHELF_LIMIT: usize = 48;

impl Store {
    pub fn load(path: PathBuf) -> Self {
        let data = match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                eprintln!("notchfy: {} inválido ({e}); usando padrões", path.display());
                Data {
                    shortcuts: default_shortcuts(),
                    shelf: Vec::new(),
                }
            }),
            Err(_) => Data {
                shortcuts: default_shortcuts(),
                shelf: Vec::new(),
            },
        };
        Self { path, data }
    }

    pub fn save(&self) -> Result<(), String> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(&self.data).map_err(|e| e.to_string())?;
        // Grava num arquivo temporário e renomeia para não corromper os dados
        // se o app fechar no meio da escrita.
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }

    pub fn snapshot(&self) -> Data {
        let mut data = self.data.clone();
        for item in &mut data.shelf {
            item.exists = Path::new(&item.path).exists();
        }
        data
    }

    pub fn shelf_add(&mut self, paths: Vec<String>) {
        let now = jiff::Timestamp::now().as_second();
        for path in paths {
            if self.data.shelf.iter().any(|i| i.path == path) {
                continue;
            }
            let p = Path::new(&path);
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone());
            self.data.shelf.insert(
                0,
                ShelfItem {
                    is_dir: p.is_dir(),
                    path,
                    name,
                    added_at: now,
                    exists: true,
                },
            );
        }
        self.data.shelf.truncate(SHELF_LIMIT);
    }
}

fn shortcut(id: &str, name: &str, icon: &str, kind: &str, target: &str) -> Shortcut {
    Shortcut {
        id: id.into(),
        name: name.into(),
        icon: icon.into(),
        kind: kind.into(),
        target: target.into(),
    }
}

fn default_shortcuts() -> Vec<Shortcut> {
    let mut list = Vec::new();
    #[cfg(target_os = "macos")]
    list.extend([
        shortcut("finder", "Finder", "📁", "app", "Finder"),
        shortcut("terminal", "Terminal", "⌘", "app", "Terminal"),
    ]);
    #[cfg(windows)]
    list.extend([
        shortcut("explorer", "Explorador", "📁", "app", "explorer.exe"),
        shortcut("terminal", "Terminal", "⌘", "app", "powershell.exe"),
    ]);
    #[cfg(target_os = "linux")]
    list.extend([
        shortcut("files", "Arquivos", "📁", "path", "~"),
        shortcut(
            "terminal",
            "Terminal",
            "⌘",
            "command",
            "x-terminal-emulator",
        ),
    ]);
    list.extend([
        shortcut("downloads", "Downloads", "⬇", "path", "~/Downloads"),
        shortcut("claude", "Claude", "✳", "url", "https://claude.ai"),
    ]);
    list
}

fn expand_home(target: &str) -> String {
    let home = || dirs::home_dir().map(|h| h.to_string_lossy().into_owned());
    if target == "~" {
        return home().unwrap_or_else(|| target.into());
    }
    match (target.strip_prefix("~/"), home()) {
        (Some(rest), Some(h)) => Path::new(&h).join(rest).to_string_lossy().into_owned(),
        _ => target.into(),
    }
}

/// Inicia o processo sem prender o app e recolhe o filho quando ele terminar.
fn spawn_detached(mut cmd: Command) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn shell(command: &str) -> Command {
    #[cfg(windows)]
    {
        let mut c = Command::new("cmd");
        c.args(["/C", command]);
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = Command::new("sh");
        c.args(["-c", command]);
        c
    }
}

fn launch_app(target: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let mut c = Command::new("open");
        c.args(["-a", target]);
        spawn_detached(c)
    }
    #[cfg(windows)]
    {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", target]);
        spawn_detached(c)
    }
    #[cfg(target_os = "linux")]
    {
        // Um id de .desktop (ex.: "firefox") abre pelo gtk-launch, que usa o
        // mesmo lançador do menu de aplicativos. Caso contrário, vira comando.
        let id = target.trim_end_matches(".desktop");
        let ok = Command::new("gtk-launch")
            .arg(id)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            Ok(())
        } else {
            spawn_detached(shell(target))
        }
    }
}

pub fn launch(app: &AppHandle, s: &Shortcut) -> Result<(), String> {
    let target = s.target.trim();
    if target.is_empty() {
        return Err("Atalho sem destino".into());
    }
    match s.kind.as_str() {
        "url" => app
            .opener()
            .open_url(target, None::<&str>)
            .map_err(|e| e.to_string()),
        "path" => app
            .opener()
            .open_path(expand_home(target), None::<&str>)
            .map_err(|e| e.to_string()),
        "app" => launch_app(target),
        "command" => spawn_detached(shell(target)),
        other => Err(format!("Tipo de atalho desconhecido: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shelf_deduplicates_and_puts_newest_first() {
        let dir = std::env::temp_dir().join("notchfy-test-store");
        let mut store = Store {
            path: dir.join("x.json"),
            data: Data::default(),
        };
        store.shelf_add(vec!["/a/um.txt".into(), "/a/dois.txt".into()]);
        store.shelf_add(vec!["/a/um.txt".into(), "/b".into()]);
        let names: Vec<_> = store.data.shelf.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["b", "dois.txt", "um.txt"]);
    }

    #[test]
    fn save_and_load_round_trip() {
        let dir = std::env::temp_dir().join(format!("notchfy-test-{}", std::process::id()));
        let path = dir.join("notchfy.json");
        let mut store = Store::load(path.clone());
        assert!(
            !store.data.shortcuts.is_empty(),
            "primeira execução cria atalhos padrão"
        );
        store.data.shortcuts = vec![shortcut("x", "X", "X", "url", "https://example.com")];
        store.save().unwrap();
        let again = Store::load(path);
        assert_eq!(again.data.shortcuts, store.data.shortcuts);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn home_expansion() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_home("~"), home.to_string_lossy());
        assert_eq!(
            expand_home("~/Downloads"),
            home.join("Downloads").to_string_lossy()
        );
        assert_eq!(expand_home("/tmp"), "/tmp");
    }
}
