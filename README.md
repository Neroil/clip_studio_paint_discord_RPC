# Clip Studio Presence

For now this app is entirely vibe-coded because of laziness. I might clean it up, improve the code,
and maintain it more seriously later, but right now it is very much a "I wanted the tool, so I made
the tool" project.

Clip Studio Presence is a small Windows desktop app that publishes Discord Rich Presence while Clip
Studio Paint is running.

## Features

- Discord Rich Presence for Clip Studio Paint.
- Focus-aware status: shows when you are drawing and when you are away/procrastinating.
- Smart activity timer:
  - while focused, Discord shows actual drawing time only;
  - while away, Discord shows how long you have been procrastinating since you last left Clip Studio.
- Optional procrastination percentage in the Discord activity text.
- Active document title support for the state line and RPC name.
- Live in-app preview of the Discord activity.
- Custom activity type, profile line, details, state, image keys, hover text, party size, and buttons.
- Screenshot sharing button:
  - captures the Clip Studio Paint window;
  - optionally applies an OBS-style PNG LUT;
  - uploads the image;
  - updates the Discord button URL at runtime.
- Optional auto screenshot sharing:
  - default first capture after 30 seconds of Clip Studio focus;
  - default repeat every 5 minutes while focus stays in Clip Studio;
  - leaving Clip Studio resets the timers.
- Windows tray behavior:
  - pressing the window X hides the app instead of closing it;
  - right-click the tray icon to open, hide, or exit.
- Optional start with Windows setting.
- Basic GitHub release update check.
- Windows installer release workflow.

## Install

Download the latest Windows setup installer from GitHub Releases and run it.

If you already installed the app, running a newer installer for the same app should install over the
existing copy. If the version number has not changed, Windows may not make it look like a normal
upgrade, but the installer should still replace the app files.

## Discord Setup

The app ships with a bundled Discord application ID, so users do not need to create their own
Discord Developer Application.

If you want full CustomRP-style control over the bold app name and asset library:

1. Open the [Discord Developer Portal](https://discord.com/developers/applications).
2. Create or open an application.
3. In **General Information**, copy the **Application ID**.
4. Set the application **Name** to the title you want Discord to show in bold.
5. Paste the ID into **Discord application ID** in the app settings.
6. Upload Rich Presence image assets.
7. Use those asset keys in the app.

The default known asset keys are:

```text
icon_1
icon_2
icon_3
```

Discord may take a few minutes to make new art assets available in Rich Presence.

## Screenshot Sharing

Use **Capture & Share** to screenshot the current Clip Studio Paint window and attach a Discord
activity button labeled **See what I'm working on**.

Discord only allows two Rich Presence buttons. When a shared screenshot URL exists, the screenshot
button uses the first slot and the app fills any remaining slot with your custom buttons.

The screenshot feature can apply an OBS-style PNG LUT before upload. The LUT must be a 512x512
OBS-style flattened PNG LUT.

## Auto Capture

Auto capture is off by default.

When enabled, the defaults are:

- first automatic capture after 30 seconds of continuous Clip Studio focus;
- repeat capture every 5 minutes after that;
- leaving Clip Studio resets the timers, so the next focused session waits 30 seconds again.

Both timings are editable in the app.

## Tray And Startup

The app is meant to keep running quietly:

- closing the window hides it to the system tray;
- the tray menu can open, hide, or fully exit the app;
- the startup setting can register the app to start with Windows.

## Updates

The app can check GitHub Releases and tell you when a newer release exists.

Fully automatic in-app installation is not implemented yet. That can be added later with signed
Tauri updater artifacts.

## Development

Install dependencies:

```powershell
npm install
```

Run the app in development:

```powershell
npm run tauri dev
```

Build the Windows NSIS installer locally:

```powershell
npm run tauri build -- --bundles nsis
```

## Releases

The GitHub release workflow builds a Windows setup installer when a version tag is pushed:

```powershell
git tag v1.0.1
git push origin v1.0.1
```

The workflow can also be started manually from GitHub Actions with a tag name.

Before tagging a new version, update these files to the same version:

- `package.json`
- `package-lock.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json`
