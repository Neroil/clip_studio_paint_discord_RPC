# Clip Studio Presence

A small Tauri + Rust desktop app that publishes Discord Rich Presence while Clip Studio Paint is running.

## Development

Install dependencies:

```powershell
npm install
```

Run the app:

```powershell
npm run tauri dev
```

You need a Discord Developer Application client ID for Rich Presence. Upload an image asset named
`clip_studio_paint` or change the asset key in the app settings.
