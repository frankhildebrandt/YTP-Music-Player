# YTP Title Catcher

Tauri desktop app for macOS and Windows that catches the current YouTube Music title from a normal browser tab and exposes OBS-friendly overlays plus a session credit list.

## Development

```bash
npm install
make run
```

## Build

```bash
make bundle
```

`make bundle` creates local macOS `app`/`dmg` bundles.

Windows `msi`/`nsis` installers are built by GitHub Actions when a GitHub Release is published. The workflow uploads the generated Windows assets to that release.

## Browser Bridge

1. Start the app.
2. Open `http://127.0.0.1:4391/bridge`.
3. Open YouTube Music in Chrome or Firefox.
4. Run the bookmarklet from the bridge page in the YouTube Music tab.

## OBS URLs

- Player overlay: `http://127.0.0.1:4391/overlay`
- Credit overlay: `http://127.0.0.1:4391/credits`
- Plain credit list: `http://127.0.0.1:4391/credits.txt`
