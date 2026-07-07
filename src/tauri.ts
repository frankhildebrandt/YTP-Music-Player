import { invoke } from "@tauri-apps/api/core";
import type { AppSettings, PlayerState } from "./types";

const fallbackSettings: AppSettings = {
  overlayPort: 4391,
};

const isTauri = "__TAURI_INTERNALS__" in window;

export async function loadSettings(): Promise<AppSettings> {
  if (!isTauri) {
    const raw = localStorage.getItem("ytp-settings");
    return raw ? { ...fallbackSettings, ...JSON.parse(raw) } : fallbackSettings;
  }
  return invoke<AppSettings>("load_settings");
}

export async function updatePlayerState(playerState: PlayerState): Promise<void> {
  if (!isTauri) {
    localStorage.setItem("ytp-player-state", JSON.stringify(playerState));
    return;
  }
  await invoke("update_player_state", { playerState });
}
