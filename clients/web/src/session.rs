use std::sync::Arc;

use leptos::{prelude::*, task::spawn_local};

use api_client::{ApiError, Client};
use protocol::Me;

const REFRESH_TOKEN_KEY: &str = "homespace.refresh_token";
const INVALID_LOGIN_MESSAGE: &str = "Invalid username or password";

thread_local! {
    static CLIENT: Client = build_client();
}

/// The API client for this origin; clones share one session.
pub fn client() -> Client {
    CLIENT.with(Client::clone)
}

fn build_client() -> Client {
    let origin = web_sys::window()
        .and_then(|window| window.location().origin().ok())
        .unwrap_or_default();
    Client::with_listener(origin, Arc::new(persist_refresh_token))
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn persist_refresh_token(token: Option<&str>) {
    if let Some(storage) = storage() {
        let _ = match token {
            Some(token) => storage.set_item(REFRESH_TOKEN_KEY, token),
            None => storage.remove_item(REFRESH_TOKEN_KEY),
        };
    }
}

fn saved_refresh_token() -> Option<String> {
    storage()?.get_item(REFRESH_TOKEN_KEY).ok().flatten()
}

/// Reactive sign-in state shared by all pages.
#[derive(Clone, Copy)]
pub struct Auth {
    pub user: RwSignal<Option<Me>>,
    pub checking: RwSignal<bool>,
}

impl Auth {
    /// Creates the state and resumes a saved session in the background.
    pub fn start() -> Self {
        let saved = saved_refresh_token();
        let auth = Self {
            user: RwSignal::new(None),
            checking: RwSignal::new(saved.is_some()),
        };
        if let Some(token) = saved {
            spawn_local(async move {
                if let Ok(me) = client().restore(&token).await {
                    auth.user.set(Some(me));
                }
                auth.checking.set(false);
            });
        }
        auth
    }

    /// Signs in; the error is a message fit for display.
    pub async fn login(self, username: &str, password: &str) -> Result<(), String> {
        match client().login(username, password).await {
            Ok(me) => {
                self.user.set(Some(me));
                Ok(())
            }
            Err(ApiError::Unauthorized) => Err(INVALID_LOGIN_MESSAGE.to_owned()),
            Err(err) => Err(err.to_string()),
        }
    }

    /// Ends the session.
    pub fn logout(self) {
        spawn_local(async move {
            let _ = client().logout().await;
            self.user.set(None);
        });
    }

    /// Turns a failed call into a message, signing out when the session is gone.
    pub fn fail(self, err: &ApiError) -> String {
        if matches!(err, ApiError::Unauthorized) {
            self.user.set(None);
        }
        err.to_string()
    }
}
