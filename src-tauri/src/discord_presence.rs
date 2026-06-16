use crate::{
    app_state::Settings,
    clip_studio::ClipStudioDetection,
};
use discord_rich_presence::{
    activity::{Activity, Assets, Timestamps},
    DiscordIpc, DiscordIpcClient,
};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default)]
pub struct PresenceState {
    pub connected: bool,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct PresenceClient {
    client: Option<DiscordIpcClient>,
    client_id: String,
    active_since: Option<i64>,
}

impl PresenceClient {
    pub fn sync(
        &mut self,
        settings: &Settings,
        detection: &ClipStudioDetection,
    ) -> PresenceState {
        if settings.client_id.trim().is_empty() {
            self.disconnect();
            return PresenceState {
                connected: false,
                error: Some("Add a Discord application ID to start Rich Presence.".to_string()),
            };
        }

        let should_show = detection.running && (!settings.only_when_focused || detection.focused);
        if !should_show {
            self.active_since = None;
            self.clear_activity();
            return PresenceState::default();
        }

        if self.client_id != settings.client_id || self.client.is_none() {
            self.disconnect();
            self.client_id = settings.client_id.clone();

            match DiscordIpcClient::new(settings.client_id.as_str()).and_then(|mut client| {
                client.connect()?;
                Ok(client)
            }) {
                Ok(client) => self.client = Some(client),
                Err(error) => {
                    return PresenceState {
                        connected: false,
                        error: Some(format!("Could not connect to Discord: {error}")),
                    };
                }
            }
        }

        if self.active_since.is_none() {
            self.active_since = Some(now_unix());
        }

        let mut activity = Activity::new().details("Drawing in Clip Studio Paint");

        let state = if settings.show_document_name {
            detection
                .document_title
                .as_deref()
                .filter(|title| !title.is_empty())
                .unwrap_or("Working on an illustration")
        } else {
            "Making art"
        };
        activity = activity.state(state);

        if settings.show_elapsed_time {
            if let Some(started_at) = self.active_since {
                activity = activity.timestamps(Timestamps::new().start(started_at));
            }
        }

        if !settings.large_image_key.trim().is_empty() {
            activity = activity.assets(
                Assets::new()
                    .large_image(settings.large_image_key.as_str())
                    .large_text("Clip Studio Paint"),
            );
        }

        match self
            .client
            .as_mut()
            .expect("presence client should be connected")
            .set_activity(activity)
        {
            Ok(()) => PresenceState {
                connected: true,
                error: None,
            },
            Err(error) => {
                self.disconnect();
                PresenceState {
                    connected: false,
                    error: Some(format!("Could not update Discord: {error}")),
                }
            }
        }
    }

    fn clear_activity(&mut self) {
        if let Some(client) = self.client.as_mut() {
            let _ = client.clear_activity();
        }
    }

    fn disconnect(&mut self) {
        self.clear_activity();
        if let Some(client) = self.client.as_mut() {
            let _ = client.close();
        }
        self.client = None;
        self.client_id.clear();
    }
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

