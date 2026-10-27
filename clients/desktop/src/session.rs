use std::sync::{Arc, mpsc};

use serde::{Deserialize, Serialize};

use api_client::{ApiError, Client};
use protocol::Me;

#[cfg(not(feature = "keyring"))]
use store_file::{forget, load, save};
#[cfg(feature = "keyring")]
use store_keyring::{forget, load, save};

/// A session kept between runs: in the system keyring with the `keyring` feature, otherwise
/// in `~/.local/share/home-space/session.json`.
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

/// A client for `server` that keeps its refresh token between runs, so the
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

#[cfg(feature = "keyring")]
mod store_keyring {
    use keyring::{Entry, Error as KeyringError};

    use super::Saved;

    const SERVICE: &str = "home-space";
    const ACCOUNT: &str = "session";

    pub fn load() -> Option<Saved> {
        let secret = Entry::new(SERVICE, ACCOUNT).ok()?.get_password().ok()?;
        serde_json::from_str(&secret).ok()
    }

    pub fn save(saved: &Saved) -> Result<(), String> {
        let secret = serde_json::to_string(saved).map_err(|err| err.to_string())?;
        Entry::new(SERVICE, ACCOUNT)
            .and_then(|entry| entry.set_password(&secret))
            .map_err(|err| err.to_string())
    }

    pub fn forget() -> Result<(), String> {
        match Entry::new(SERVICE, ACCOUNT).and_then(|entry| entry.delete_credential()) {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }
}

#[cfg(not(feature = "keyring"))]
mod store_file {
    use std::io::{ErrorKind, Write};
    use std::path::PathBuf;

    use super::Saved;

    /// `~/.local/share/home-space/session.json`.
    fn path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(PathBuf::from(home).join(".local/share/home-space/session.json"))
    }

    pub fn load() -> Option<Saved> {
        let json = std::fs::read(path()?).ok()?;
        serde_json::from_slice(&json).ok()
    }

    pub fn save(saved: &Saved) -> Result<(), String> {
        let path = path().ok_or("HOME is not set")?;
        let json = serde_json::to_vec(saved).map_err(|err| err.to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|err| err.to_string())?;
        }
        // Write a sibling and rename it over, so a crash never leaves half a session behind.
        let tmp = path.with_extension("json.tmp");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let mut file = options.open(&tmp).map_err(|err| err.to_string())?;
        file.write_all(&json).map_err(|err| err.to_string())?;
        std::fs::rename(&tmp, &path).map_err(|err| err.to_string())
    }

    pub fn forget() -> Result<(), String> {
        let Some(path) = path() else { return Ok(()) };
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.to_string()),
        }
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
