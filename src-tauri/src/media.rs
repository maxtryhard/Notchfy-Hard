//! O que está tocando agora, com uma implementação por sistema:
//! - Linux: MPRIS pelo D-Bus (Spotify, navegadores, VLC, Rhythmbox…).
//! - Windows: System Media Transport Controls (qualquer app que apareça no
//!   painel de mídia do Windows).
//! - macOS: AppleScript no Spotify e no Música. O MediaRemote, que cobriria
//!   qualquer player, foi fechado para apps de terceiros no macOS 15.4.

use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaState {
    pub available: bool,
    pub playing: bool,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub player: String,
    /// URL https ou data URL pronta para um `<img>`.
    pub art: Option<String>,
    pub position: Option<f64>,
    pub duration: Option<f64>,
}

pub fn state() -> MediaState {
    imp::state()
}

pub fn control(action: &str) -> Result<(), String> {
    match action {
        "play_pause" | "next" | "previous" => imp::control(action),
        other => Err(format!("Ação desconhecida: {other}")),
    }
}

/// A capa só muda quando a faixa muda; guardamos a última para não reler o
/// arquivo (ou o stream do Windows) a cada atualização.
#[cfg(not(target_os = "macos"))]
static ART_CACHE: std::sync::Mutex<Option<(String, Option<String>)>> = std::sync::Mutex::new(None);

#[cfg(not(target_os = "macos"))]
fn cached_art(key: &str, load: impl FnOnce() -> Option<String>) -> Option<String> {
    let mut cache = ART_CACHE.lock().unwrap();
    if let Some((k, v)) = cache.as_ref() {
        if k == key {
            return v.clone();
        }
    }
    let value = load();
    *cache = Some((key.to_string(), value.clone()));
    value
}

#[cfg(not(target_os = "macos"))]
fn data_url(mime: &str, bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine};
    format!("data:{mime};base64,{}", STANDARD.encode(bytes))
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use mpris::{PlaybackStatus, PlayerFinder};

    const MAX_ART_BYTES: u64 = 4 * 1024 * 1024;

    fn art_from_url(url: &str) -> Option<String> {
        if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("data:") {
            return Some(url.to_string());
        }
        let path = url.strip_prefix("file://")?;
        let path = percent_decode(path);
        // O caminho vem de qualquer player no D-Bus: só aceitamos imagens,
        // para um player malicioso não fazer o app ler outros arquivos.
        let mime = match path.rsplit('.').next()?.to_ascii_lowercase().as_str() {
            "png" => "image/png",
            "webp" => "image/webp",
            "jpg" | "jpeg" => "image/jpeg",
            _ => return None,
        };
        if std::fs::metadata(&path).ok()?.len() > MAX_ART_BYTES {
            return None;
        }
        Some(data_url(mime, &std::fs::read(&path).ok()?))
    }

    fn percent_decode(s: &str) -> String {
        let bytes = s.as_bytes();
        let mut out = Vec::with_capacity(bytes.len());
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let Ok(b) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(b);
                    i += 3;
                    continue;
                }
            }
            out.push(bytes[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }

    pub fn state() -> MediaState {
        let Ok(finder) = PlayerFinder::new() else {
            return MediaState::default();
        };
        let Ok(player) = finder.find_active() else {
            return MediaState::default();
        };
        let Ok(meta) = player.get_metadata() else {
            return MediaState::default();
        };
        let title = meta.title().unwrap_or_default().to_string();
        let artist = meta.artists().map(|a| a.join(", ")).unwrap_or_default();
        let art = meta
            .art_url()
            .and_then(|url| cached_art(url, || art_from_url(url)));
        MediaState {
            available: true,
            playing: player.get_playback_status().ok() == Some(PlaybackStatus::Playing),
            album: meta.album_name().unwrap_or_default().to_string(),
            player: player.identity().to_string(),
            position: player.get_position().ok().map(|d| d.as_secs_f64()),
            duration: meta.length().map(|d| d.as_secs_f64()),
            title,
            artist,
            art,
        }
    }

    pub fn control(action: &str) -> Result<(), String> {
        let player = PlayerFinder::new()
            .map_err(|e| e.to_string())?
            .find_active()
            .map_err(|e| e.to_string())?;
        match action {
            "play_pause" => player.play_pause(),
            "next" => player.next(),
            _ => player.previous(),
        }
        .map_err(|e| e.to_string())
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn decodes_file_urls() {
            assert_eq!(
                super::percent_decode("/home/a%20b/capa%C3%A7.jpg"),
                "/home/a b/capaç.jpg"
            );
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows::{
        Media::Control::{
            GlobalSystemMediaTransportControlsSession as Session,
            GlobalSystemMediaTransportControlsSessionManager as Manager,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
        },
        Storage::Streams::DataReader,
    };

    const MAX_ART_BYTES: u64 = 4 * 1024 * 1024;

    fn session() -> windows::core::Result<Session> {
        Manager::RequestAsync()?.join()?.GetCurrentSession()
    }

    fn thumbnail(s: &Session) -> windows::core::Result<Option<String>> {
        let props = s.TryGetMediaPropertiesAsync()?.join()?;
        let Ok(reference) = props.Thumbnail() else {
            return Ok(None);
        };
        let stream = reference.OpenReadAsync()?.join()?;
        let size = stream.Size()?;
        if size == 0 || size > MAX_ART_BYTES {
            return Ok(None);
        }
        let reader = DataReader::CreateDataReader(&stream)?;
        reader.LoadAsync(size as u32)?.join()?;
        let mut buf = vec![0u8; size as usize];
        reader.ReadBytes(&mut buf)?;
        let mime = stream
            .ContentType()
            .map(|c| c.to_string())
            .unwrap_or_default();
        let mime = if mime.starts_with("image/") {
            mime
        } else {
            "image/png".into()
        };
        Ok(Some(data_url(&mime, &buf)))
    }

    fn read() -> windows::core::Result<MediaState> {
        let s = session()?;
        let props = s.TryGetMediaPropertiesAsync()?.join()?;
        let info = s.GetPlaybackInfo()?;
        let timeline = s.GetTimelineProperties().ok();
        let title = props.Title()?.to_string();
        let artist = props.Artist()?.to_string();
        let key = format!("{}|{}|{}", s.SourceAppUserModelId()?, title, artist);
        let ticks = |t: windows::Foundation::TimeSpan| t.Duration as f64 / 10_000_000.0;
        Ok(MediaState {
            available: true,
            playing: info.PlaybackStatus()? == Status::Playing,
            album: props.AlbumTitle()?.to_string(),
            player: s.SourceAppUserModelId()?.to_string(),
            art: cached_art(&key, || thumbnail(&s).ok().flatten()),
            position: timeline.as_ref().and_then(|t| t.Position().ok()).map(ticks),
            duration: timeline
                .as_ref()
                .and_then(|t| t.EndTime().ok())
                .map(ticks)
                .filter(|d| *d > 0.0),
            title,
            artist,
        })
    }

    pub fn state() -> MediaState {
        read().unwrap_or_default()
    }

    pub fn control(action: &str) -> Result<(), String> {
        let s = session().map_err(|e| e.to_string())?;
        let op = match action {
            "play_pause" => s.TryTogglePlayPauseAsync(),
            "next" => s.TrySkipNextAsync(),
            _ => s.TrySkipPreviousAsync(),
        };
        op.and_then(|o| o.join())
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use std::process::Command;

    const SEP: &str = "␟";

    fn running(process: &str) -> bool {
        Command::new("pgrep")
            .args(["-xq", process])
            .status()
            .is_ok_and(|s| s.success())
    }

    /// Só monta o script do app que já está aberto: compilar um `tell` para um
    /// app que não está instalado faz o macOS abrir um diálogo "Onde está…?".
    fn active_player() -> Option<&'static str> {
        ["Spotify", "Music"].into_iter().find(|p| running(p))
    }

    fn osascript(script: &str) -> Option<String> {
        let out = Command::new("osascript")
            .args(["-e", script])
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn number(s: &str) -> Option<f64> {
        // O AppleScript formata números com a vírgula decimal do idioma do sistema.
        s.trim().replace(',', ".").parse().ok()
    }

    pub fn state() -> MediaState {
        let Some(app) = active_player() else {
            return MediaState::default();
        };
        // A duração vem em milissegundos no Spotify e em segundos no Música.
        let (art, duration) = if app == "Spotify" {
            (
                "artwork url of current track",
                "(duration of current track) / 1000",
            )
        } else {
            ("\"\"", "duration of current track")
        };
        let script = format!(
            r#"tell application "{app}"
  if player state is stopped then return ""
  set s to "{SEP}"
  return (player state as string) & s & (name of current track) & s & (artist of current track) & s & (album of current track) & s & {art} & s & ({duration}) & s & player position
end tell"#
        );
        let Some(out) = osascript(&script).filter(|o| !o.is_empty()) else {
            return MediaState::default();
        };
        let f: Vec<&str> = out.split(SEP).collect();
        if f.len() < 7 {
            return MediaState::default();
        }
        MediaState {
            available: true,
            playing: f[0] == "playing",
            title: f[1].into(),
            artist: f[2].into(),
            album: f[3].into(),
            player: if app == "Music" {
                "Música".into()
            } else {
                app.into()
            },
            art: Some(f[4].to_string()).filter(|a| a.starts_with("https://")),
            duration: number(f[5]),
            position: number(f[6]),
        }
    }

    pub fn control(action: &str) -> Result<(), String> {
        let app = active_player().ok_or("Nenhum player aberto")?;
        let verb = match action {
            "play_pause" => "playpause",
            "next" => "next track",
            _ => "previous track",
        };
        osascript(&format!(r#"tell application "{app}" to {verb}"#))
            .map(|_| ())
            .ok_or_else(|| "O AppleScript falhou (permissão de Automação?)".into())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod imp {
    use super::*;
    pub fn state() -> MediaState {
        MediaState::default()
    }
    pub fn control(_: &str) -> Result<(), String> {
        Err("Sistema sem suporte a controle de mídia".into())
    }
}
