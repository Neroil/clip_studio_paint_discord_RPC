use crate::clip_studio::{detect_clip_studio, ClipStudioDetection};
use crate::discord_presence::PresenceClient;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Settings {
    pub client_id: String,
    pub large_image_key: String,
    pub show_document_name: bool,
    pub show_elapsed_time: bool,
    pub only_when_focused: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            large_image_key: "clip_studio_paint".to_string(),
            show_document_name: true,
            show_elapsed_time: true,
            only_when_focused: false,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct AppStatus {
    pub settings: Settings,
    pub clip_studio_running: bool,
    pub clip_studio_focused: bool,
    pub document_title: Option<String>,
    pub discord_connected: bool,
    pub discord_error: Option<String>,
    pub last_updated_unix: u64,
}

#[derive(Clone, Debug)]
struct RuntimeState {
    settings: Settings,
    detection: ClipStudioDetection,
    discord_connected: bool,
    discord_error: Option<String>,
    last_updated_unix: u64,
}

pub struct AppState {
    inner: Arc<Mutex<RuntimeState>>,
    config_path: PathBuf,
}

impl AppState {
    pub fn load(app: AppHandle) -> Self {
        let config_path = config_path(&app);
        let settings = fs::read_to_string(&config_path)
            .ok()
            .and_then(|json| serde_json::from_str::<Settings>(&json).ok())
            .unwrap_or_default();

        Self {
            inner: Arc::new(Mutex::new(RuntimeState {
                settings,
                detection: ClipStudioDetection::default(),
                discord_connected: false,
                discord_error: None,
                last_updated_unix: now_unix(),
            })),
            config_path,
        }
    }

    pub fn snapshot(&self) -> AppStatus {
        let inner = self.inner.lock().expect("app state lock poisoned");

        AppStatus {
            settings: inner.settings.clone(),
            clip_studio_running: inner.detection.running,
            clip_studio_focused: inner.detection.focused,
            document_title: inner.detection.document_title.clone(),
            discord_connected: inner.discord_connected,
            discord_error: inner.discord_error.clone(),
            last_updated_unix: inner.last_updated_unix,
        }
    }

    pub fn save_settings(&self, settings: Settings) -> Result<(), SaveSettingsError> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(&settings)?;
        fs::write(&self.config_path, json)?;

        let mut inner = self.inner.lock().expect("app state lock poisoned");
        inner.settings = settings;
        Ok(())
    }

    pub fn spawn_monitor(&self) {
        let state = self.clone_for_thread();

        thread::spawn(move || {
            let mut presence = PresenceClient::default();

            loop {
                let settings = {
                    let inner = state.inner.lock().expect("app state lock poisoned");
                    inner.settings.clone()
                };

                let detection = detect_clip_studio();
                let presence_state = presence.sync(&settings, &detection);

                {
                    let mut inner = state.inner.lock().expect("app state lock poisoned");
                    inner.detection = detection;
                    inner.discord_connected = presence_state.connected;
                    inner.discord_error = presence_state.error;
                    inner.last_updated_unix = now_unix();
                }

                thread::sleep(Duration::from_secs(3));
            }
        });
    }

    fn clone_for_thread(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            config_path: self.config_path.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SaveSettingsError {
    #[error("could not write settings file: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not serialize settings: {0}")]
    Json(#[from] serde_json::Error),
}

fn config_path(app: &AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("settings.json")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
