mod client;
mod genres;
mod playback;
mod subscriptions;
mod trim;
mod wire;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context as _, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use ytmusic::{Tokens, YtMusic, oauth};

use crate::youtube::playback::Factory;

use crate::{
    InputSource, MusicProvider, PromptSink, ProviderSession, SignIn, SignInPrompt, UserProfile,
    credentials,
};
pub use client::YouTubeClient;

const GUEST_ID: &str = "youtube-guest";

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Saved {
    OAuth { tokens: Tokens },
    Guest,
}

pub struct YouTubeProvider {
    credentials: PathBuf,
    tokens: PathBuf,
    resolved: PathBuf,
    player: PathBuf,
}

impl YouTubeProvider {
    pub fn new() -> Self {
        let cache = credentials::dir("youtube");
        Self {
            credentials: cache.join(credentials::FILE),
            tokens: cache.join("oauth.json"),
            resolved: cache.join("resolved.json"),
            player: cache.join("player.json"),
        }
    }

    fn save(&self, saved: &Saved) -> Result<()> {
        save(&self.credentials, saved)
    }

    fn saved(&self) -> Option<Saved> {
        let body = std::fs::read(&self.credentials).ok()?;
        match serde_json::from_slice(&body) {
            Ok(saved) => Some(saved),
            Err(error) => {
                log::warn!("youtube: cannot read the stored credentials: {error}");
                credentials::remove(&self.credentials);
                None
            }
        }
    }

    fn oauth_client(&self, tokens: Tokens) -> Arc<YtMusic> {
        Arc::new(
            YtMusic::new(tokens)
                .persist_to(self.tokens.clone())
                .cache_resolutions(self.resolved.clone())
                .cache_player(self.player.clone()),
        )
    }

    fn guest_client(&self) -> Arc<YtMusic> {
        Arc::new(YtMusic::anonymous().cache_player(self.player.clone()))
    }

    fn authenticated_session(&self, api: Arc<YtMusic>, profile: UserProfile) -> ProviderSession {
        let client = YouTubeClient::new(api.clone()).owned_by(profile.display_name.clone());
        ProviderSession {
            profile,
            api: Arc::new(client),
            playback: Arc::new(Factory::new(api)),
            authenticated: true,
            playcounts: false,
        }
    }

    fn guest_session(&self, api: Arc<YtMusic>) -> ProviderSession {
        ProviderSession {
            profile: UserProfile {
                id: GUEST_ID.to_string(),
                display_name: "YouTube Music".to_string(),
            },
            api: Arc::new(YouTubeClient::new(api.clone())),
            playback: Arc::new(Factory::new(api)),
            authenticated: false,
            playcounts: false,
        }
    }

    async fn connect_oauth(&self, prompt: &PromptSink) -> Result<ProviderSession> {
        let http = reqwest::Client::new();
        let identity = oauth::fetch_identity(&http).await;
        let device = oauth::request_device_code(&http, &identity)
            .await
            .context("cannot start youtube authorization")?;
        prompt(SignInPrompt::Code {
            code: device.user_code.clone(),
            url: device.verification_url.clone(),
        });
        let tokens = oauth::poll_token(&http, &identity, &device)
            .await
            .context("youtube authorization failed")?;
        let api = self.oauth_client(tokens.clone());
        let profile = api
            .profile()
            .await
            .context("youtube did not accept the authorized session")?;
        if let Err(error) = tokens.save(&self.tokens) {
            log::warn!("youtube: cannot cache the refreshable tokens: {error:#}");
        }
        credentials::secure(&self.tokens);
        self.save(&Saved::OAuth { tokens })
            .context("cannot store youtube oauth credentials")?;
        log::debug!("youtube: oauth sign-in succeeded");
        Ok(self.authenticated_session(api, wire::profile(profile)))
    }

    async fn restore_oauth(&self, tokens: Tokens) -> Option<ProviderSession> {
        let tokens = match Tokens::load(&self.tokens) {
            Ok(Some(refreshed)) => refreshed,
            _ => tokens,
        };
        let api = self.oauth_client(tokens);
        match api.profile().await {
            Ok(profile) => {
                log::debug!("youtube: restored the oauth session");
                Some(self.authenticated_session(api, wire::profile(profile)))
            }
            Err(error) => {
                log::warn!("youtube: the cached oauth credentials are no longer usable: {error:#}");
                None
            }
        }
    }

    fn store_guest(&self) {
        if let Err(error) = self.save(&Saved::Guest) {
            log::warn!("youtube: cannot remember the guest session: {error:#}");
        }
    }
}

pub(crate) fn migrate() {
    let cache = credentials::dir("youtube");
    let file = cache.join(credentials::FILE);
    let cookies = cache.join("cookies.txt");
    let authuser = cache.join("authuser.txt");
    let guest = cache.join("guest");
    if !file.exists()
        && guest.exists()
        && let Err(error) = save(&file, &Saved::Guest)
    {
        log::warn!("youtube: cannot adopt the old guest marker: {error:#}");
        return;
    }
    for path in [&cookies, &authuser, &guest] {
        credentials::remove(path);
    }
}

fn save(file: &std::path::Path, saved: &Saved) -> Result<()> {
    let body = serde_json::to_vec_pretty(saved).context("cannot encode youtube credentials")?;
    credentials::write(file, &body)
}

impl Default for YouTubeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MusicProvider for YouTubeProvider {
    fn name(&self) -> &'static str {
        "YouTube Music"
    }

    fn slug(&self) -> &'static str {
        "youtube"
    }

    fn sign_in_options(&self) -> Vec<SignIn> {
        vec![SignIn::Default, SignIn::Anonymous]
    }

    fn stored(&self) -> bool {
        self.credentials.exists()
    }

    async fn restore(&self) -> Result<Option<ProviderSession>> {
        match self.saved() {
            Some(Saved::OAuth { tokens }) => Ok(self.restore_oauth(tokens).await),
            Some(Saved::Guest) => {
                log::debug!("youtube: restoring guest session");
                Ok(Some(self.guest_session(self.guest_client())))
            }
            None => Ok(None),
        }
    }

    async fn sign_in(
        &self,
        method: SignIn,
        prompt: crate::PromptSink,
        _input: InputSource,
    ) -> Result<ProviderSession> {
        match method {
            SignIn::Default => self.connect_oauth(&prompt).await,
            SignIn::Anonymous => {
                self.store_guest();
                Ok(self.guest_session(self.guest_client()))
            }
            SignIn::Path(_) => Err(anyhow::anyhow!(
                "youtube does not sign in with a folder path"
            )),
        }
    }

    fn sign_out(&self) {
        credentials::remove(&self.credentials);
        credentials::remove(&self.tokens);
    }
}
