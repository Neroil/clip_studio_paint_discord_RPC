<div align="center">

# Clip Studio Presence

**Discord Rich Presence for Clip Studio Paint — because your friends deserve to know you've been staring at the same lineart for 3 hours :3**

![Platform](https://img.shields.io/badge/platform-Windows-blue)
![Release](https://img.shields.io/github/v/release/Neroil/clip_studio_paint_discord_RPC)
![Downloads](https://img.shields.io/github/downloads/Neroil/clip_studio_paint_discord_RPC/total)

</div>

---

> ⚠️ **Vibe-coded with love.** This started as a "I REALLY WANT DISCORD RICH PRESENCE FOR CLIP STUDIO" project. The code is... working. Mhhhyeah! I might clean it up and maintain it more seriously over time, but no promises teehee

---

## What is this?

Clip Studio Presence is a small Windows desktop app that shows your Clip Studio Paint session on your Discord profile as a Rich Presence activity. It shows how long you've been drawing, and whether you're actually drawing or just procrastinating.

## Features

### Core Rich Presence
- Live Discord Rich Presence while Clip Studio Paint is open
- **Focus-aware status** — shows *"Drawing"* when CSP is focused, *"Procrastinating"* when you've wandered off
- **Smart timer** — drawing time only counts while you're actually in CSP; away timer tracks how long you've been gone since you last left
- Optional **procrastination percentage** in the activity text (for the brave and the honest)
- Live **in-app preview** of what your Discord activity looks like (super badly done)

### Customization
- Custom activity type, profile line, details, state, image keys, hover text, party size, and buttons
- Bring your own Discord Application ID to customize the images shown to your FRIENDS

### Screenshot Sharing
- **Capture & Share** button — screenshots your CSP window and attaches a *"See what I'm working on"* button to your Discord activity
- Uploads the picture on [Uguu~](https://uguu.se)
- Optional OBS-style PNG LUT support (512×512 flattened PNG)
- **Auto capture** mode. First shot after 30s of focus, then every 5 minutes while you stay in CSP

### Quality of Life
- Runs quietly in the **system tray**, closing the window hides it, doesn't quit it
- Right-click tray icon to open, hide, or exit
- Optional **start with Windows** setting
- Built-in **update checker** against GitHub Releases (Haven't tested it yet)

---

## Install

1. Go to the [**Releases**](../../releases/latest) page
2. Download the latest `ClipStudioPresence-x.x.x-setup.exe`
3. Run it and follow the installer

---

## Discord Setup

The app ships with a bundled Discord application ID. Just install and go, no setup required.

If you want to use your own Discord application (for a custom bold name and your own asset library):

1. Go to the [Discord Developer Portal](https://discord.com/developers/applications)
2. Create or open an application
3. Copy the **Application ID** from General Information
4. Set the application **Name** to whatever you want Discord to show in bold
5. Paste the ID into **Discord application ID** in the app settings
6. Upload your Rich Presence image assets
7. Use those asset keys in the app

The built-in asset keys are: `icon_1`, `icon_2`, `icon_3`

---

## Screenshot Sharing

Hit **Capture & Share** to screenshot the current CSP window and add a clickable button to your Discord activity.

Discord only allows 2 Rich Presence buttons sooo when a screenshot URL is active, it takes slot 1, and your custom buttons fill any remaining slot.

Optionally apply an OBS-style PNG LUT (must be a 512×512 flattened PNG) before the image is uploaded.

---

## Development

**Prerequisites:** [Node.js](https://nodejs.org/) + [Rust](https://rustup.rs/)

```powershell
# Install dependencies
npm install

# Run in development
npm run tauri dev

# Build Windows NSIS installer
npm run tauri build -- --bundles nsis
```

---

## Releasing

The GitHub Actions workflow builds a Windows setup installer on tag push:

```powershell
git tag v1.0.1
git push origin v1.0.1
```

You can also trigger it manually from the Actions tab.

Before tagging, make sure these four files are all on the same version:
- `package.json`
- `package-lock.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`

---

## Built with

- [Tauri](https://tauri.app/) — Rust + web frontend desktop framework
- [discord-rich-presence](https://crates.io/crates/discord-rich-presence) — RPC crate
- JavaScript / HTML / CSS frontend
- ChatGPT's codex cause I had a free trial thank god

---
