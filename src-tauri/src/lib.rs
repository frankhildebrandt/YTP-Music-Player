use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    AppHandle, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tiny_http::{Header, Method, Response, Server};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    title: String,
    artist: String,
    album: String,
    position: String,
    duration: String,
    progress_percent: f64,
    artwork_url: String,
    song_url: String,
    is_playing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTrack {
    title: String,
    artist: String,
    album: String,
    duration: String,
    artwork_url: String,
    song_url: String,
    first_seen_at: String,
    last_seen_at: String,
    play_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    overlay_port: u16,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            overlay_port: 4391,
        }
    }
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            title: "YouTube Music".to_string(),
            artist: "Bereit zum Abspielen".to_string(),
            album: String::new(),
            position: String::new(),
            duration: String::new(),
            progress_percent: 0.0,
            artwork_url: String::new(),
            song_url: String::new(),
            is_playing: false,
        }
    }
}

#[derive(Clone)]
struct SharedState {
    settings: Arc<Mutex<AppSettings>>,
    player: Arc<Mutex<PlayerState>>,
    session_history: Arc<Mutex<Vec<SessionTrack>>>,
}

fn app_dir(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| anyhow::anyhow!(err.to_string()))?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn settings_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    Ok(app_dir(app)?.join("settings.json"))
}

fn read_settings(app: &AppHandle) -> anyhow::Result<AppSettings> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }
    let data = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&data).unwrap_or_default())
}

fn youtube_now_playing_init_script(port: u16) -> String {
    format!(
        r##"
(() => {{
  if (!window.location.hostname.endsWith("youtube.com")) return;

  const endpoint = "http://127.0.0.1:{port}/api/now-playing";
  let lastPayload = "";
  const shareUrlByKey = new Map();
  const pendingShareKeys = new Set();

  const text = (root, selector) => {{
    const node = root.querySelector(selector);
    return node?.textContent?.replace(/\s+/g, " ").trim() || "";
  }};

  const attr = (root, selector, name) => {{
    const node = root.querySelector(selector);
    return node?.getAttribute(name) || "";
  }};

  const firstText = (root, selectors) => {{
    for (const selector of selectors) {{
      const value = text(root, selector);
      if (value) return value;
    }}
    return "";
  }};

  const cleanImageUrl = (url) => {{
    if (!url) return "";
    const cleaned = String(url).trim();
    if (!cleaned || cleaned.startsWith("data:") || cleaned.startsWith("blob:")) return "";
    return cleaned
      .replace(/=w\d+-h\d+(?:-[\w]+)*/i, "=w512-h512")
      .replace(/=s\d+(?:-[\w]+)*/i, "=w512-h512");
  }};

  const imageFromSrcset = (srcset) => {{
    if (!srcset) return "";
    const entries = String(srcset)
      .split(",")
      .map((entry) => {{
        const parts = entry.trim().split(/\s+/);
        const width = Number((parts[1] || "").replace(/[^\d.]/g, "")) || 0;
        return {{ url: parts[0] || "", width }};
      }})
      .filter((entry) => entry.url);
    entries.sort((a, b) => b.width - a.width);
    return entries[0]?.url || "";
  }};

  const backgroundImageUrl = (node) => {{
    if (!node) return "";
    const image = window.getComputedStyle(node).backgroundImage || "";
    const match = image.match(/url\((['"]?)(.*?)\1\)/i);
    return match?.[2] || "";
  }};

  const nodeImageUrl = (node) => cleanImageUrl(
    node?.currentSrc ||
    node?.src ||
    node?.getAttribute?.("src") ||
    imageFromSrcset(node?.getAttribute?.("srcset")) ||
    node?.getAttribute?.("data-src") ||
    node?.getAttribute?.("data-thumb") ||
    node?.getAttribute?.("data-thumbnail") ||
    backgroundImageUrl(node)
  );

  const artworkFrom = (root) => {{
    const selectors = [
      "img.image",
      "yt-img-shadow img",
      "#thumbnail img",
      ".thumbnail img",
      ".thumbnail-image-wrapper img",
      "img"
    ];
    for (const selector of selectors) {{
      const nodes = [...root.querySelectorAll(selector)]
        .filter((node) => visible(node))
        .sort((a, b) => (b.naturalWidth || b.width || 0) * (b.naturalHeight || b.height || 0) - (a.naturalWidth || a.width || 0) * (a.naturalHeight || a.height || 0));
      for (const node of nodes) {{
        const url = nodeImageUrl(node);
        if (url) return url;
      }}
    }}

    for (const node of root.querySelectorAll("[style*='background-image']")) {{
      const url = nodeImageUrl(node);
      if (url) return url;
    }}
    return "";
  }};

  const cleanSongUrl = (rawUrl) => {{
    if (!rawUrl) return "";
    try {{
      const url = new URL(rawUrl, window.location.href);
      const videoId = url.hostname === "youtu.be"
        ? url.pathname.replace(/^\/+/, "").split("/")[0]
        : url.searchParams.get("v");
      if (!videoId) return "";
      return `https://music.youtube.com/watch?v=${{encodeURIComponent(videoId)}}`;
    }} catch {{
      return "";
    }}
  }};

  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
  const visible = (node) => {{
    if (!node) return false;
    const rect = node.getBoundingClientRect();
    const style = window.getComputedStyle(node);
    return rect.width > 0 && rect.height > 0 && style.visibility !== "hidden" && style.display !== "none";
  }};
  const clickNode = (node) => {{
    if (!node || !visible(node)) return false;
    node.dispatchEvent(new MouseEvent("pointerdown", {{ bubbles: true, cancelable: true, view: window }}));
    node.dispatchEvent(new MouseEvent("mousedown", {{ bubbles: true, cancelable: true, view: window }}));
    node.click();
    node.dispatchEvent(new MouseEvent("mouseup", {{ bubbles: true, cancelable: true, view: window }}));
    return true;
  }};
  const clickFirstVisible = (selectors, root = document) => {{
    for (const selector of selectors) {{
      for (const node of root.querySelectorAll(selector)) {{
        if (clickNode(node)) return true;
      }}
    }}
    return false;
  }};
  const clickShareMenuItem = () => {{
    const items = [
      ...document.querySelectorAll("ytmusic-menu-navigation-item-renderer, ytmusic-menu-service-item-renderer, tp-yt-paper-item, ytd-menu-service-item-renderer")
    ];
    for (const item of items) {{
      const label = item.textContent?.replace(/\s+/g, " ").trim() || "";
      if (/^(share|teilen)$/i.test(label) || /\b(share|teilen)\b/i.test(label)) {{
        if (clickNode(item)) return true;
      }}
    }}
    return false;
  }};
  const readShareDialogUrl = () => {{
    const candidates = [
      ...document.querySelectorAll("input, textarea")
    ].map((node) => node.value || node.getAttribute("value") || "");
    candidates.push(...[...document.querySelectorAll("a[href]")].map((node) => node.getAttribute("href") || ""));
    for (const value of candidates) {{
      const cleaned = cleanSongUrl(value);
      if (cleaned) return cleaned;
    }}
    return "";
  }};
  const closeShareDialog = () => {{
    clickFirstVisible([
      "ytmusic-popup-container tp-yt-paper-icon-button[aria-label*='Close' i]",
      "ytmusic-popup-container tp-yt-paper-icon-button[aria-label*='Schlie' i]",
      "tp-yt-paper-dialog tp-yt-paper-icon-button[aria-label*='Close' i]",
      "tp-yt-paper-dialog tp-yt-paper-icon-button[aria-label*='Schlie' i]",
      "ytmusic-popup-container button[aria-label*='Close' i]",
      "ytmusic-popup-container button[aria-label*='Schlie' i]"
    ]);
    document.dispatchEvent(new KeyboardEvent("keydown", {{ key: "Escape", code: "Escape", bubbles: true }}));
  }};
  const openCurrentSongMenu = () => {{
    const bar = document.querySelector("ytmusic-player-bar") || document;
    return clickFirstVisible([
      "tp-yt-paper-icon-button[aria-label*='More' i]",
      "tp-yt-paper-icon-button[aria-label*='Menu' i]",
      "tp-yt-paper-icon-button[aria-label*='Aktion' i]",
      "tp-yt-paper-icon-button[aria-label*='Weitere' i]",
      "button[aria-label*='More' i]",
      "button[aria-label*='Menu' i]",
      "button[aria-label*='Aktion' i]",
      "button[aria-label*='Weitere' i]",
      "#button-shape button",
      ".menu button",
      ".menu tp-yt-paper-icon-button"
    ], bar);
  }};
  const trackKey = (state) => `${{state.artist || ""}}|${{state.title || ""}}`.toLowerCase();
  const resolveShareUrl = async (state) => {{
    const key = trackKey(state);
    if (!key.trim() || shareUrlByKey.has(key) || pendingShareKeys.has(key)) return;
    pendingShareKeys.add(key);
    try {{
      if (!openCurrentSongMenu()) return;
      await sleep(300);
      if (!clickShareMenuItem()) return;
      for (let attempt = 0; attempt < 12; attempt += 1) {{
        await sleep(250);
        const shareUrl = readShareDialogUrl();
        if (shareUrl) {{
          shareUrlByKey.set(key, shareUrl);
          closeShareDialog();
          send(true);
          return;
        }}
      }}
      closeShareDialog();
    }} finally {{
      closeShareDialog();
      pendingShareKeys.delete(key);
    }}
  }};

  const readState = () => {{
    const bar = document.querySelector("ytmusic-player-bar") || document;
    const title = firstText(bar, [
      ".title",
      "yt-formatted-string.title",
      "#layout .title",
      "ytmusic-player-queue-item[selected] .song-title"
    ]);
    const byline = firstText(bar, [
      ".byline",
      "yt-formatted-string.byline",
      ".subtitle",
      "#layout .subtitle"
    ]);
    const parts = byline.split(/ • | · | - /).map((part) => part.trim()).filter(Boolean);
    const artist = parts[0] || "";
    const album = parts.length > 1 ? parts.slice(1).join(" · ") : "";
    const timeInfo = firstText(bar, [
      ".time-info",
      "span.time-info"
    ]);
    const position = firstText(bar, [
      ".time-info .current-time",
      ".time-info .elapsed",
      ".current-time",
      ".elapsed",
      "#progress-bar #left-bound-time"
    ]) || timeInfo.replace(/\s*\/.*$/, "");
    const duration = firstText(bar, [
      ".time-info .duration",
      ".duration",
      "#progress-bar #right-bound-time",
      "span.time-info"
    ]).replace(/^.*\/\s*/, "");
    const progressNode = bar.querySelector("input[type='range'], [role='slider'], tp-yt-paper-slider, #progress-bar [role='slider'], #progress-bar");
    const progressValue = Number(progressNode?.getAttribute?.("aria-valuenow") || progressNode?.value || 0);
    const progressMax = Number(progressNode?.getAttribute?.("aria-valuemax") || progressNode?.max || 0);
    const progressPercent = progressMax > 0
      ? Math.min(100, Math.max(0, (progressValue / progressMax) * 100))
      : 0;
    const artworkUrl = artworkFrom(bar) || artworkFrom(document.querySelector("ytmusic-player") || document);
    const playButtonLabel = attr(bar, "#play-pause-button", "title") || attr(bar, "#play-pause-button", "aria-label");
    const isPlaying = /pause|pausieren/i.test(playButtonLabel);
    const baseState = {{
      title,
      artist,
      album,
      position,
      duration,
      progressPercent,
      artworkUrl,
      isPlaying
    }};

    return {{
      ...baseState,
      songUrl: shareUrlByKey.get(trackKey(baseState)) || "",
      isPlaying: baseState.isPlaying
    }};
  }};

  const send = (force = false) => {{
    const state = readState();
    if (!state.title && !state.artist) return;
    if (!state.songUrl) resolveShareUrl(state);
    const payload = JSON.stringify(state);
    if (!force && payload === lastPayload) return;
    lastPayload = payload;
    fetch(endpoint, {{
      method: "POST",
      mode: "no-cors",
      body: payload
    }}).catch(() => {{
      const beacon = new Image();
      beacon.src = `${{endpoint}}?data=${{encodeURIComponent(payload)}}&t=${{Date.now()}}`;
    }});
  }};

  setInterval(send, 1000);
  new MutationObserver(send).observe(document.documentElement, {{
    childList: true,
    subtree: true,
    characterData: true,
    attributes: true
  }});
  window.addEventListener("load", send);
  send();
}})();
"##
    )
}

#[tauri::command]
fn load_settings(app: AppHandle, state: tauri::State<SharedState>) -> Result<AppSettings, String> {
    let settings = read_settings(&app).map_err(|err| err.to_string())?;
    *state.settings.lock().map_err(|err| err.to_string())? = settings.clone();
    Ok(settings)
}

#[tauri::command]
fn update_player_state(
    player_state: PlayerState,
    state: tauri::State<SharedState>,
) -> Result<(), String> {
    if let Ok(mut history) = state.session_history.lock() {
        record_session_track(&mut history, &player_state);
    }
    *state.player.lock().map_err(|err| err.to_string())? = player_state;
    Ok(())
}

fn open_settings_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("settings") {
        window.set_focus().map_err(|err| err.to_string())?;
        return Ok(());
    }

    WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("index.html#settings".into()),
    )
    .title("Einstellungen")
    .inner_size(840.0, 680.0)
    .min_inner_size(680.0, 540.0)
    .build()
    .map_err(|err| err.to_string())?;

    Ok(())
}

#[tauri::command]
fn open_settings(app: AppHandle) -> Result<(), String> {
    open_settings_window(&app)
}


fn json_response<T: Serialize>(value: &T) -> Response<io::Cursor<Vec<u8>>> {
    let body = serde_json::to_vec(value).unwrap_or_default();
    Response::from_data(body)
        .with_header(
            Header::from_bytes("Content-Type", "application/json; charset=utf-8")
                .expect("valid static header"),
        )
        .with_header(
            Header::from_bytes("Access-Control-Allow-Origin", "*").expect("valid static header"),
        )
}

fn text_response(text: String, content_type: &'static str) -> Response<io::Cursor<Vec<u8>>> {
    Response::from_string(text)
        .with_header(Header::from_bytes("Content-Type", content_type).expect("valid static header"))
        .with_header(
            Header::from_bytes("Access-Control-Allow-Origin", "*").expect("valid static header"),
        )
}

fn html_response(html: String) -> Response<io::Cursor<Vec<u8>>> {
    Response::from_string(html).with_header(
        Header::from_bytes("Content-Type", "text/html; charset=utf-8")
            .expect("valid static header"),
    )
}

fn now_isoish() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    seconds.to_string()
}

fn track_key(title: &str, artist: &str) -> String {
    format!(
        "{}|{}",
        title.trim().to_lowercase(),
        artist.trim().to_lowercase()
    )
}

fn should_record_track(player: &PlayerState) -> bool {
    let title = player.title.trim();
    let artist = player.artist.trim();
    !title.is_empty()
        && !artist.is_empty()
        && title != "YouTube Music"
        && title != "Bereit"
        && artist != "Bereit zum Abspielen"
        && artist != "YouTube Music öffnen"
}

fn record_session_track(history: &mut Vec<SessionTrack>, player: &PlayerState) {
    if !should_record_track(player) {
        return;
    }

    let key = track_key(&player.title, &player.artist);
    let timestamp = now_isoish();
    if let Some(last) = history.last_mut() {
        if track_key(&last.title, &last.artist) == key {
            last.album = player.album.clone();
            last.duration = player.duration.clone();
            last.artwork_url = player.artwork_url.clone();
            last.song_url = player.song_url.clone();
            last.last_seen_at = timestamp;
            return;
        }
    }

    if let Some(existing) = history
        .iter_mut()
        .find(|track| track_key(&track.title, &track.artist) == key)
    {
        existing.album = player.album.clone();
        existing.duration = player.duration.clone();
        existing.artwork_url = player.artwork_url.clone();
        existing.song_url = player.song_url.clone();
        existing.last_seen_at = timestamp;
        existing.play_count = existing.play_count.saturating_add(1);
        return;
    }

    history.push(SessionTrack {
        title: player.title.clone(),
        artist: player.artist.clone(),
        album: player.album.clone(),
        duration: player.duration.clone(),
        artwork_url: player.artwork_url.clone(),
        song_url: player.song_url.clone(),
        first_seen_at: timestamp.clone(),
        last_seen_at: timestamp,
        play_count: 1,
    });
}

fn render_credits_text(history: &[SessionTrack]) -> String {
    history
        .iter()
        .map(|track| {
            let mut line = format!("{} - {}", track.artist, track.title);
            if !track.album.trim().is_empty() {
                line.push_str(&format!(" ({})", track.album));
            }
            if !track.duration.trim().is_empty() {
                line.push_str(&format!(" [{}]", track.duration));
            }
            if !track.song_url.trim().is_empty() {
                line.push_str(&format!(" - {}", track.song_url));
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn start_overlay_server(shared: SharedState) {
    let port = shared
        .settings
        .lock()
        .ok()
        .map(|settings| settings.overlay_port)
        .unwrap_or(4391);

    thread::spawn(move || {
        let Ok(server) = Server::http(("127.0.0.1", port)) else {
            eprintln!("Overlay server could not bind to 127.0.0.1:{port}");
            return;
        };

        for mut request in server.incoming_requests() {
            let url = request.url().to_string();
            let method = request.method().clone();
            let mut body = String::new();
            if method == Method::Post {
                let _ = request
                    .as_reader()
                    .take(64 * 1024)
                    .read_to_string(&mut body);
            }
            let response = route_overlay(&shared, method, &url, &body);
            let _ = request.respond(response);
        }
    });
}

fn route_overlay(
    shared: &SharedState,
    method: Method,
    url: &str,
    body: &str,
) -> Response<io::Cursor<Vec<u8>>> {
    if method != Method::Get && method != Method::Post {
        return Response::from_string("Method not allowed").with_status_code(405);
    }

    if url == "/overlay" || url == "/" {
        let player = shared
            .player
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default();
        return html_response(render_overlay(&player));
    }

    if url == "/api/state" {
        let player = shared
            .player
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default();
        return json_response(&player);
    }

    if url == "/api/session-history" {
        let history = shared
            .session_history
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default();
        return json_response(&history);
    }

    if method == Method::Post && url == "/api/session-history/reset" {
        if let Ok(mut history) = shared.session_history.lock() {
            history.clear();
        }
        return json_response(&Vec::<SessionTrack>::new());
    }

    if url == "/credits.txt" {
        let history = shared
            .session_history
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default();
        return text_response(render_credits_text(&history), "text/plain; charset=utf-8");
    }

    if url == "/credits" {
        let history = shared
            .session_history
            .lock()
            .map(|state| state.clone())
            .unwrap_or_default();
        return html_response(render_credits(&history));
    }

    if url == "/bridge" {
        let port = shared
            .settings
            .lock()
            .map(|settings| settings.overlay_port)
            .unwrap_or(4391);
        return html_response(render_bridge(port));
    }

    if (method == Method::Post && url == "/api/now-playing")
        || url.starts_with("/api/now-playing?data=")
    {
        let payload = if let Some(encoded) = url.strip_prefix("/api/now-playing?data=") {
            let encoded = encoded.split('&').next().unwrap_or(encoded);
            urlencoding::decode(encoded)
                .map(|decoded| decoded.into_owned())
                .unwrap_or_default()
        } else {
            body.to_string()
        };

        if let Ok(incoming) = serde_json::from_str::<PlayerStatePatch>(&payload) {
            if let Ok(mut player) = shared.player.lock() {
                if let Some(title) = clean_optional(incoming.title) {
                    player.title = title;
                }
                if let Some(artist) = clean_optional(incoming.artist) {
                    player.artist = artist;
                }
                if let Some(album) = clean_optional(incoming.album) {
                    player.album = album;
                }
                if let Some(position) = clean_optional(incoming.position) {
                    player.position = position;
                }
                if let Some(duration) = clean_optional(incoming.duration) {
                    player.duration = duration;
                }
                if let Some(progress_percent) = incoming.progress_percent {
                    player.progress_percent = progress_percent.clamp(0.0, 100.0);
                }
                if let Some(artwork_url) = clean_optional(incoming.artwork_url) {
                    player.artwork_url = artwork_url;
                }
                if let Some(song_url) = clean_optional(incoming.song_url) {
                    player.song_url = song_url;
                }
                if let Some(is_playing) = incoming.is_playing {
                    player.is_playing = is_playing;
                }
                if let Ok(mut history) = shared.session_history.lock() {
                    record_session_track(&mut history, &player);
                }
                if method == Method::Post {
                    return json_response(&*player);
                }
                return tiny_png_response();
            }
        }
        return Response::from_string("Invalid now playing payload").with_status_code(400);
    }

    Response::from_string("Not found").with_status_code(404)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerStatePatch {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    position: Option<String>,
    duration: Option<String>,
    progress_percent: Option<f64>,
    artwork_url: Option<String>,
    song_url: Option<String>,
    is_playing: Option<bool>,
}

fn clean_optional(value: Option<String>) -> Option<String> {
    let cleaned = value?.trim().to_string();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn tiny_png_response() -> Response<io::Cursor<Vec<u8>>> {
    const TRANSPARENT_PNG: &[u8] = &[
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ];
    Response::from_data(TRANSPARENT_PNG.to_vec())
        .with_header(Header::from_bytes("Content-Type", "image/png").expect("valid static header"))
}

fn render_overlay(player: &PlayerState) -> String {
    let artwork = if player.artwork_url.is_empty() {
        r#"<div class="art fallback"></div>"#.to_string()
    } else {
        format!(
            r#"<img class="art" src="{}" alt="">"#,
            html_escape(&player.artwork_url)
        )
    };
    let initial_artwork =
        serde_json::to_string(&player.artwork_url).unwrap_or_else(|_| "\"\"".to_string());
    let overlay_class = if player.is_playing {
        "overlay"
    } else {
        "overlay is-hidden"
    };
    format!(
        r#"<!doctype html>
<html lang="de">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>YTP Overlay</title>
  <style>
    :root {{ --stage-padding: clamp(12px, 1.25vw, 32px); --art-size: clamp(58px, 5vw, 92px); }}
    html, body {{ margin: 0; width: 100vw; height: 100vh; overflow: hidden; background: transparent; font-family: Inter, system-ui, sans-serif; color: white; }}
    body {{ position: relative; }}
    .overlay {{ box-sizing: border-box; position: absolute; top: var(--stage-padding); right: var(--stage-padding); display: inline-flex; align-items: center; gap: clamp(10px, .75vw, 16px); width: max-content; max-width: min(25vw, calc(100vw - (var(--stage-padding) * 2))); padding: clamp(10px, .75vw, 16px) clamp(12px, .85vw, 18px); border-radius: 8px; background: rgba(12, 14, 16, .78); opacity: 1; transform: translateX(0) scale(1); filter: blur(0); transition: opacity 420ms ease, transform 420ms ease, filter 420ms ease; }}
    .overlay.is-hidden {{ opacity: 0; transform: translateX(28px) scale(.985); filter: blur(3px); }}
    #art-slot {{ display: block; width: var(--art-size); height: var(--art-size); flex: 0 0 var(--art-size); overflow: hidden; border-radius: 8px; background: linear-gradient(135deg, #ff5964, #ffd166 52%, #06d6a0); }}
    .art {{ display: block; width: 100%; height: 100%; border-radius: inherit; object-fit: cover; background: linear-gradient(135deg, #ff5964, #ffd166 52%, #06d6a0); }}
    .copy {{ width: clamp(120px, 16vw, 320px); min-width: 0; flex: 1 1 auto; }}
    .scroll-line {{ display: block; width: 100%; overflow: hidden; white-space: nowrap; }}
    .scroll-line > span {{ display: inline-block; min-width: 100%; transform: translateX(0); }}
    .scroll-line.is-scrolling > span {{ padding-right: 44px; animation: scrollText var(--scroll-duration, 18s) linear 1.2s infinite; }}
    h1 {{ margin: 0 0 7px; font-size: clamp(18px, 1.55vw, 30px); line-height: 1.06; }}
    p {{ margin: 0; color: #f2f6f8; font-size: clamp(13px, .9vw, 17px); }}
    .meta {{ margin-top: 8px; color: #8bd4ff; font-size: clamp(11px, .75vw, 14px); }}
    .progress {{ position: relative; height: clamp(3px, .22vw, 5px); margin-top: clamp(8px, .55vw, 12px); overflow: hidden; border-radius: 999px; background: rgba(255, 255, 255, .18); }}
    .progress-fill {{ position: absolute; inset: 0 auto 0 0; width: 0%; border-radius: inherit; background: linear-gradient(90deg, #ff5964, #ffd166 58%, #06d6a0); transition: width 700ms linear; }}
    @keyframes scrollText {{
      0%, 12% {{ transform: translateX(0); }}
      88%, 100% {{ transform: translateX(calc(var(--scroll-distance, 0px) * -1)); }}
    }}
  </style>
</head>
<body>
  <div class="{}" id="overlay">
    <div id="art-slot">{}</div>
    <div class="copy">
      <h1 class="scroll-line"><span id="title">{}</span></h1>
      <p class="scroll-line"><span id="artist">{}</span></p>
      <div class="meta scroll-line"><span id="meta">{}</span></div>
      <div class="progress" aria-hidden="true"><div class="progress-fill" id="progress-fill"></div></div>
    </div>
  </div>
  <script>
    const stateUrl = "/api/state";
    let currentArtwork = {};

    const setText = (id, value) => {{
      const node = document.getElementById(id);
      const next = value || "";
      if (node && node.textContent !== next) {{
        node.textContent = next;
        requestAnimationFrame(() => updateScrollState(node.parentElement));
      }}
    }};

    const updateScrollState = (line) => {{
      if (!line) return;
      const text = line.firstElementChild;
      if (!text) return;

      line.classList.remove("is-scrolling");
      text.style.removeProperty("--scroll-distance");
      text.style.removeProperty("--scroll-duration");

      requestAnimationFrame(() => {{
        const distance = text.scrollWidth - line.clientWidth;
        if (distance <= 1) return;
        text.style.setProperty("--scroll-distance", `${{distance + 44}}px`);
        text.style.setProperty("--scroll-duration", `${{Math.max(16, (distance + 44) / 14)}}s`);
        line.classList.add("is-scrolling");
      }});
    }};

    const updateAllScrollStates = () => {{
      document.querySelectorAll(".scroll-line").forEach(updateScrollState);
    }};

    const formatMeta = (duration, album) => {{
      if (duration && album) return `${{duration}} · ${{album}}`;
      return duration || album || "";
    }};

    const parseTime = (value) => {{
      const parts = String(value || "")
        .trim()
        .split(":")
        .map((part) => Number(part.replace(/[^\d.]/g, "")))
        .filter((part) => Number.isFinite(part));
      if (!parts.length) return 0;
      return parts.reduce((total, part) => (total * 60) + part, 0);
    }};

    const setProgress = (position, duration, progressPercent) => {{
      const fill = document.getElementById("progress-fill");
      if (!fill) return;
      const currentSeconds = parseTime(position);
      const durationSeconds = parseTime(duration);
      const directPercent = Number(progressPercent);
      const percent = Number.isFinite(directPercent) && directPercent > 0
        ? Math.min(100, Math.max(0, directPercent))
        : durationSeconds > 0
        ? Math.min(100, Math.max(0, (currentSeconds / durationSeconds) * 100))
        : 0;
      fill.style.width = `${{percent}}%`;
    }};

    const setPlaybackVisible = (isPlaying) => {{
      const overlay = document.getElementById("overlay");
      if (!overlay) return;
      overlay.classList.toggle("is-hidden", !isPlaying);
    }};

    const setArtwork = (url) => {{
      const next = url || "";
      if (next === currentArtwork) return;
      currentArtwork = next;

      const slot = document.getElementById("art-slot");
      if (!slot) return;
      if (!next) {{
        slot.innerHTML = '<div class="art fallback"></div>';
        return;
      }}

      const img = new Image();
      img.className = "art";
      img.alt = "";
      img.onload = () => slot.replaceChildren(img);
      img.src = next;
    }};

    const refresh = async () => {{
      try {{
        const response = await fetch(stateUrl, {{ cache: "no-store" }});
        if (!response.ok) return;
        const state = await response.json();
        setText("title", state.title || "");
        setText("artist", state.artist || "");
        setText("meta", formatMeta(state.duration || "", state.album || ""));
        setProgress(state.position || "", state.duration || "", state.progressPercent);
        setArtwork(state.artworkUrl || "");
        setPlaybackVisible(Boolean(state.isPlaying));
      }} catch {{
      }}
    }};

    window.setInterval(refresh, 1000);
    window.addEventListener("resize", updateAllScrollStates);
    refresh();
    updateAllScrollStates();
  </script>
</body>
</html>"#,
        overlay_class,
        artwork,
        html_escape(&player.title),
        html_escape(&player.artist),
        html_escape(&format_duration_album(&player.duration, &player.album)),
        initial_artwork
    )
}

fn render_credits(history: &[SessionTrack]) -> String {
    let items = if history.is_empty() {
        r#"<li class="empty">Noch keine Tracks in dieser Session.</li>"#.to_string()
    } else {
        history
            .iter()
            .map(|track| {
                let meta = format_duration_album(&track.duration, &track.album);
                let play_count = if track.play_count > 1 {
                    format!(r#"<span class="count">{}x</span>"#, track.play_count)
                } else {
                    String::new()
                };
                let link = if track.song_url.trim().is_empty() {
                    String::new()
                } else {
                    format!(
                        r#"<a class="song-link" href="{}">Song öffnen</a>"#,
                        html_escape(&track.song_url)
                    )
                };
                format!(
                    r#"<li><div><strong>{}</strong><span>{}</span><small>{}</small>{}</div>{}</li>"#,
                    html_escape(&track.title),
                    html_escape(&track.artist),
                    html_escape(&meta),
                    link,
                    play_count
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    format!(
        r#"<!doctype html>
<html lang="de">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta http-equiv="refresh" content="5">
  <title>NCS Credits</title>
  <style>
    html, body {{ margin: 0; background: transparent; font-family: Inter, system-ui, sans-serif; color: white; }}
    .credits {{ width: min(760px, 100vw); padding: 18px; border-radius: 8px; background: rgba(12, 14, 16, .78); }}
    h1 {{ margin: 0 0 14px; font-size: 28px; line-height: 1.1; }}
    ol {{ display: grid; gap: 10px; margin: 0; padding: 0 0 0 28px; }}
    li {{ padding: 8px 0; }}
    strong, span, small {{ display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }}
    strong {{ font-size: 18px; }}
    span {{ color: #f2f6f8; }}
    small {{ margin-top: 3px; color: #8bd4ff; }}
    .song-link {{ display: inline-block; margin-top: 5px; color: #8bd4ff; font-size: 13px; text-decoration: none; }}
    .count {{ display: inline-flex; margin-top: 5px; padding: 2px 8px; border-radius: 999px; color: #101214; background: #06d6a0; font-size: 12px; font-weight: 800; }}
    .empty {{ list-style: none; color: #aeb9c0; }}
  </style>
</head>
<body>
  <section class="credits">
    <h1>NCS Credits</h1>
    <ol>{}</ol>
  </section>
</body>
</html>"#,
        items
    )
}

fn render_bridge(port: u16) -> String {
    let script = youtube_now_playing_init_script(port);
    let bookmarklet = format!("javascript:{}", script.replace('\n', " "));
    format!(
        r#"<!doctype html>
<html lang="de">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>YouTube Music Bridge</title>
  <style>
    html, body {{ margin: 0; min-height: 100%; color: #eef2f5; background: #111315; font-family: Inter, system-ui, sans-serif; }}
    main {{ width: min(860px, calc(100vw - 32px)); margin: 0 auto; padding: 32px 0; }}
    section {{ border: 1px solid rgba(255,255,255,.12); border-radius: 8px; padding: 18px; background: rgba(20,23,26,.86); }}
    h1 {{ margin: 0 0 12px; font-size: 28px; }}
    p, li {{ color: #c9d2d7; line-height: 1.5; }}
    a.button {{ display: inline-flex; align-items: center; min-height: 44px; margin: 8px 0 16px; border-radius: 8px; padding: 0 18px; color: #fff; background: #e34d59; font-weight: 800; text-decoration: none; }}
    textarea {{ width: 100%; min-height: 260px; border: 1px solid rgba(255,255,255,.14); border-radius: 8px; padding: 12px; color: #eef2f5; background: #101214; font: 13px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }}
    code {{ color: #8bd4ff; }}
  </style>
</head>
<body>
  <main>
    <section>
      <h1>YouTube Music Bridge</h1>
      <p>Öffne <code>music.youtube.com</code> in deinem normalen Browser, während diese App läuft. Ziehe den Button in deine Lesezeichenleiste oder kopiere den Code in die Browser-Konsole auf dem YouTube-Music-Tab.</p>
      <a class="button" href="{}">YTP Bridge starten</a>
      <textarea spellcheck="false">{}</textarea>
      <ol>
        <li>YouTube Music im Browser öffnen und anmelden.</li>
        <li>Bookmarklet anklicken oder den Code in der Konsole ausführen.</li>
        <li>OBS Overlay: <code>http://127.0.0.1:{}/overlay</code></li>
        <li>Credits: <code>http://127.0.0.1:{}/credits.txt</code></li>
      </ol>
    </section>
  </main>
</body>
</html>"#,
        html_escape(&bookmarklet),
        html_escape(&script),
        port,
        port
    )
}

fn format_duration_album(duration: &str, album: &str) -> String {
    match (duration.is_empty(), album.is_empty()) {
        (true, true) => String::new(),
        (false, true) => duration.to_string(),
        (true, false) => album.to_string(),
        (false, false) => format!("{duration} · {album}"),
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .menu(|app| {
            let settings_item = MenuItem::with_id(
                app,
                "open_settings",
                "Einstellungen...",
                true,
                Some("CmdOrCtrl+;"),
            )?;

            #[cfg(target_os = "macos")]
            let app_menu = Submenu::with_items(
                app,
                env!("CARGO_PKG_NAME"),
                true,
                &[
                    &settings_item,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::services(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::hide(app, None)?,
                    &PredefinedMenuItem::hide_others(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, None)?,
                ],
            )?;

            #[cfg(not(target_os = "macos"))]
            let app_menu = Submenu::with_items(
                app,
                "App",
                true,
                &[
                    &settings_item,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::quit(app, None)?,
                ],
            )?;

            let edit_menu = Submenu::with_items(
                app,
                "Edit",
                true,
                &[
                    &PredefinedMenuItem::undo(app, None)?,
                    &PredefinedMenuItem::redo(app, None)?,
                    &PredefinedMenuItem::separator(app)?,
                    &PredefinedMenuItem::cut(app, None)?,
                    &PredefinedMenuItem::copy(app, None)?,
                    &PredefinedMenuItem::paste(app, None)?,
                    &PredefinedMenuItem::select_all(app, None)?,
                ],
            )?;

            Menu::with_items(app, &[&app_menu, &edit_menu])
        })
        .on_menu_event(|app, event| {
            if event.id() == "open_settings" {
                if let Err(err) = open_settings_window(app) {
                    eprintln!("Could not open settings window: {err}");
                }
            }
        })
        .setup(|app| {
            let settings = read_settings(app.handle()).unwrap_or_default();
            let shared = SharedState {
                player: Arc::new(Mutex::new(PlayerState::default())),
                settings: Arc::new(Mutex::new(settings)),
                session_history: Arc::new(Mutex::new(Vec::new())),
            };
            start_overlay_server(shared.clone());
            app.manage(shared);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_settings,
            update_player_state,
            open_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
