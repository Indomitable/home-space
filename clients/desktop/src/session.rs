use std::sync::{Arc, mpsc};

use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};

use api_client::{ApiError, Client};
use protocol::Me;

const SERVICE: &str = "home-space";
const ACCOUNT: &str = "session";

/// A session kept in the system keyring between runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Saved {
    pub server: String,
    pub refresh_token: String,
}

/// The result of resuming the saved session at startup.
#[derive(Debug, Clone)]
pub enum Restored {
    Nothing,
    SignedIn { client: Client, me: Me },
    Failed { server: String, error: ApiError },
}

/// A client for `server` that keeps its refresh token in the system keyring, so the
/// next start can resume the session; signing out removes it.
pub fn client(server: &str) -> Client {
    let (sender, receiver) = mpsc::channel::<Option<String>>();
    let server = server.to_owned();
    let base_url = server.clone();
    std::thread::spawn(move || persist(&server, &receiver));
    Client::with_listener(
        base_url,
        Arc::new(move |token: Option<&str>| {
            let _ = sender.send(token.map(str::to_owned));
        }),
    )
}

/// Resumes the session saved by an earlier run, if any.
pub async fn restore() -> Restored {
    match tokio::task::spawn_blocking(load).await.ok().flatten() {
        None => Restored::Nothing,
        Some(saved) => {
            let client = client(&saved.server);
            match client.restore(&saved.refresh_token).await {
                Ok(me) => Restored::SignedIn { client, me },
                Err(error) => Restored::Failed {
                    server: saved.server,
                    error,
                },
            }
        }
    }
}

fn persist(server: &str, tokens: &mpsc::Receiver<Option<String>>) {
    for token in tokens {
        let result = match token {
            Some(refresh_token) => save(&Saved {
                server: server.to_owned(),
                refresh_token,
            }),
            None => forget(),
        };
        if let Err(err) = result {
            eprintln!("could not update the saved session: {err}");
        }
    }
}

fn load() -> Option<Saved> {
    let secret = Entry::new(SERVICE, ACCOUNT).ok()?.get_password().ok()?;
    serde_json::from_str(&secret).ok()
}

fn save(saved: &Saved) -> Result<(), String> {
    let secret = serde_json::to_string(saved).map_err(|err| err.to_string())?;
    Entry::new(SERVICE, ACCOUNT)
        .and_then(|entry| entry.set_password(&secret))
        .map_err(|err| err.to_string())
}

fn forget() -> Result<(), String> {
    match Entry::new(SERVICE, ACCOUNT).and_then(|entry| entry.delete_credential()) {
        Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_session_round_trips_through_json() {
        let saved = Saved {
            server: "http://127.0.0.1:8080".to_owned(),
            refresh_token: "token".to_owned(),
        };

        let json = serde_json::to_string(&saved).expect("serializes");

        assert_eq!(serde_json::from_str::<Saved>(&json).expect("parses"), saved);
    }
}
