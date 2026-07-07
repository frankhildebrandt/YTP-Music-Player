import { useEffect, useState } from "react";
import { Activity, Clock3, Copy, Disc3, KeyRound, Music2, Trash2 } from "lucide-react";
import { Toaster, toast } from "sonner";
import type { AppSettings, PlayerState, SessionTrack } from "./types";
import { loadSettings } from "./tauri";

const defaultSettings: AppSettings = {
  overlayPort: 4391,
};

const defaultPlayerState: PlayerState = {
  title: "Bereit",
  artist: "Bridge starten",
  album: "",
  position: "",
  duration: "",
  progressPercent: 0,
  artworkUrl: "",
  songUrl: "",
  isPlaying: false,
};

function errorDescription(error: unknown) {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  if (error && typeof error === "object") return JSON.stringify(error);
  return "Unbekannter Fehler";
}

function notifyError(title: string, error: unknown) {
  console.error(title, error);
  toast.error(title, {
    description: errorDescription(error),
  });
}

function samePlayerState(current: PlayerState, next: PlayerState) {
  return (
    current.title === next.title &&
    current.artist === next.artist &&
    current.album === next.album &&
    current.position === next.position &&
    current.duration === next.duration &&
    current.progressPercent === next.progressPercent &&
    current.artworkUrl === next.artworkUrl &&
    current.songUrl === next.songUrl &&
    current.isPlaying === next.isPlaying
  );
}

export default function App() {
  const isSettingsWindow = window.location.hash === "#settings";
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [player, setPlayer] = useState<PlayerState>(defaultPlayerState);
  const [sessionHistory, setSessionHistory] = useState<SessionTrack[]>([]);

  useEffect(() => {
    loadSettings()
      .then((loaded) => setSettings({ ...defaultSettings, ...loaded }))
      .catch((error) => notifyError("Einstellungen konnten nicht geladen werden", error));
  }, []);

  useEffect(() => {
    let cancelled = false;
    async function pollPlayerState() {
      try {
        const response = await fetch(`http://127.0.0.1:${settings.overlayPort}/api/state`);
        if (response.ok) {
          const next = (await response.json()) as PlayerState;
          if (!cancelled) setPlayer((current) => (samePlayerState(current, next) ? current : { ...current, ...next }));
        }

        const historyResponse = await fetch(`http://127.0.0.1:${settings.overlayPort}/api/session-history`);
        if (historyResponse.ok) {
          const history = (await historyResponse.json()) as SessionTrack[];
          if (!cancelled) setSessionHistory(history);
        }
      } catch {
        // The overlay server only exists in the Tauri runtime.
      }
    }
    pollPlayerState();
    const interval = window.setInterval(pollPlayerState, 1000);
    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, [settings.overlayPort]);

  function creditsText() {
    return sessionHistory
      .map((track) => {
        const album = track.album ? ` (${track.album})` : "";
        const duration = track.duration ? ` [${track.duration}]` : "";
        const link = track.songUrl ? ` - ${track.songUrl}` : "";
        return `${track.artist || "Unbekannter Artist"} - ${track.title}${album}${duration}${link}`;
      })
      .join("\n");
  }

  async function copyCredits() {
    try {
      const text = creditsText();
      if (!text.trim()) {
        toast.error("Noch keine Session-Tracks vorhanden");
        return;
      }
      await navigator.clipboard.writeText(text);
      toast.success("Credits kopiert");
    } catch (error) {
      notifyError("Credits konnten nicht kopiert werden", error);
    }
  }

  async function resetCredits() {
    try {
      const response = await fetch(`http://127.0.0.1:${settings.overlayPort}/api/session-history/reset`, { method: "POST" });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      setSessionHistory([]);
      toast.success("Credit-Liste geleert");
    } catch (error) {
      notifyError("Credit-Liste konnte nicht geleert werden", error);
    }
  }

  const overlayUrl = `http://127.0.0.1:${settings.overlayPort}/overlay`;
  const bridgeUrl = `http://127.0.0.1:${settings.overlayPort}/bridge`;
  const creditsUrl = `http://127.0.0.1:${settings.overlayPort}/credits`;
  const creditsTextUrl = `http://127.0.0.1:${settings.overlayPort}/credits.txt`;

  function renderSettings() {
    return (
      <section className="settings-page settings-stack">
        <div className="settings-grid">
          <article className="panel">
            <header>
              <Activity size={20} />
              <h2>Overlay</h2>
            </header>
            <div className="endpoint-list">
              <a className="wide-link" href={overlayUrl} target="_blank" rel="noreferrer">
                {overlayUrl}
              </a>
              <a className="wide-link" href={creditsUrl} target="_blank" rel="noreferrer">
                {creditsUrl}
              </a>
              <a className="wide-link" href={creditsTextUrl} target="_blank" rel="noreferrer">
                {creditsTextUrl}
              </a>
            </div>
          </article>

          <article className="panel">
            <header>
              <KeyRound size={20} />
              <h2>Bridge</h2>
            </header>
            <a className="wide-link" href={bridgeUrl} target="_blank" rel="noreferrer">
              {bridgeUrl}
            </a>
            <ol className="bridge-steps">
              <li>YouTube Music im Browser öffnen.</li>
              <li>Bridge-Seite öffnen und Bookmarklet starten.</li>
              <li>Danach füllt der Browser diese App und die Overlays.</li>
            </ol>
          </article>
        </div>
      </section>
    );
  }

  if (isSettingsWindow) {
    return (
      <main className="shell settings-shell">
        <Toaster richColors closeButton position="top-right" />
        {renderSettings()}
      </main>
    );
  }

  return (
    <main className="shell">
      <Toaster richColors closeButton position="top-right" />
      <section className="player-band title-catcher-band">
        <div className="player-info">
          <div className="album-art">
            {player.artworkUrl ? <img src={player.artworkUrl} alt="" /> : <Music2 size={52} />}
          </div>
          <div>
            <p className="eyebrow">{player.isPlaying ? "Playing" : "Title Catcher"}</p>
            <h1>{player.title || "Bereit"}</h1>
            <p>{player.artist || "Bridge starten"}</p>
            <div className="track-meta">
              <span>
                <Disc3 size={16} />
                {player.album || "Kein Album"}
              </span>
              <span>
                <Clock3 size={16} />
                {player.duration || "Keine Länge"}
              </span>
            </div>
          </div>
        </div>
      </section>

      <section className="workspace">
        <article className="panel credits-panel">
          <header>
            <div className="section-title-row">
              <h2>Session Credits</h2>
              <span>{sessionHistory.length}</span>
            </div>
          </header>
          <div className="credit-actions">
            <button className="secondary-button compact-button" onClick={copyCredits}>
              <Copy size={17} />
              Kopieren
            </button>
            <button className="secondary-button compact-button" onClick={resetCredits}>
              <Trash2 size={17} />
              Leeren
            </button>
          </div>
          <div className="endpoint-list credit-links">
            <a className="wide-link" href={creditsUrl} target="_blank" rel="noreferrer">
              {creditsUrl}
            </a>
            <a className="wide-link" href={creditsTextUrl} target="_blank" rel="noreferrer">
              {creditsTextUrl}
            </a>
          </div>
          <div className="credits-list">
            {sessionHistory.map((track, index) => (
              <div className="current-track credit-track" key={`${track.title}-${track.artist}-${index}`}>
                {track.artworkUrl ? <img src={track.artworkUrl} alt="" /> : <div className="track-art-placeholder" />}
                <div>
                  <strong>{track.title}</strong>
                  <span>{track.artist || "Unbekannter Artist"}</span>
                  <small>{[track.duration, track.album, track.playCount > 1 ? `${track.playCount}x` : ""].filter(Boolean).join(" · ")}</small>
                  {track.songUrl && (
                    <a className="song-link" href={track.songUrl} target="_blank" rel="noreferrer">
                      Song öffnen
                    </a>
                  )}
                </div>
              </div>
            ))}
            {sessionHistory.length === 0 && <p className="empty">Noch keine Tracks für die Credit-Liste.</p>}
          </div>
        </article>
      </section>

      <footer>
        <span>Title-Daten kommen über die Browser-Bridge.</span>
      </footer>
    </main>
  );
}
