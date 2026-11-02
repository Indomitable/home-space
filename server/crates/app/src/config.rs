use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result};
use clap::Args;

const MIN_SECRET_LEN: usize = 32;

/// Settings shared by all subcommands, read from flags or environment.
#[derive(Args, Debug)]
pub struct Config {
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,

    /// Secret that signs access tokens; set this or `HOMESPACE_JWT_SECRET_FILE`.
    #[arg(long, env = "HOMESPACE_JWT_SECRET", hide_env_values = true)]
    pub jwt_secret: Option<String>,

    /// File holding the signing secret (e.g. a Docker/Podman secret).
    #[arg(long, env = "HOMESPACE_JWT_SECRET_FILE")]
    pub jwt_secret_file: Option<PathBuf>,

    /// Earlier signing secret that still verifies tokens, so the secret can be rotated
    /// without ending sessions. Remove it once the access tokens it signed have expired.
    #[arg(long, env = "HOMESPACE_JWT_SECRET_PREVIOUS", hide_env_values = true)]
    pub jwt_secret_previous: Option<String>,

    #[arg(long, env = "HOMESPACE_STORAGE_ROOT", default_value = "./data")]
    pub storage_root: PathBuf,

    #[arg(long, env = "HOMESPACE_BIND", default_value = "127.0.0.1:8080")]
    pub bind: SocketAddr,

    #[arg(long, env = "HOMESPACE_WEB_ROOT")]
    pub web_root: Option<PathBuf>,

    #[arg(long, env = "HOMESPACE_MAX_UPLOAD_BYTES", default_value_t = 10 * 1024 * 1024 * 1024)]
    pub max_upload_bytes: u64,

    #[arg(long, env = "HOMESPACE_ACCESS_TTL_SECS", default_value_t = 900)]
    pub access_ttl_secs: i64,

    #[arg(long, env = "HOMESPACE_REFRESH_TTL_SECS", default_value_t = 30 * 24 * 3600)]
    pub refresh_ttl_secs: i64,

    /// How long the change journal keeps entries; clients offline for longer resync fully.
    #[arg(long, env = "HOMESPACE_CHANGES_RETENTION_DAYS", default_value_t = 30)]
    pub changes_retention_days: i64,
}

impl Config {
    /// Rejects settings that would make the server insecure.
    pub fn validate(&self) -> Result<()> {
        self.jwt_secrets()
            .map(drop)
            .context("invalid configuration")
    }

    /// The current signing secret followed by the previous one, if any.
    pub fn jwt_secrets(&self) -> Result<(Vec<u8>, Vec<Vec<u8>>)> {
        let current = match (&self.jwt_secret, &self.jwt_secret_file) {
            (Some(_), Some(_)) => {
                anyhow::bail!("set only one of HOMESPACE_JWT_SECRET and HOMESPACE_JWT_SECRET_FILE")
            }
            (Some(secret), None) => secret.clone(),
            (None, Some(path)) => std::fs::read_to_string(path)
                .with_context(|| format!("reading {}", path.display()))?
                .trim_end_matches(['\r', '\n'])
                .to_owned(),
            (None, None) => {
                anyhow::bail!("HOMESPACE_JWT_SECRET or HOMESPACE_JWT_SECRET_FILE must be set")
            }
        };
        check_secret_len("the JWT secret", &current)?;
        let previous = match self
            .jwt_secret_previous
            .as_ref()
            .filter(|secret| !secret.is_empty())
        {
            Some(secret) => {
                check_secret_len("HOMESPACE_JWT_SECRET_PREVIOUS", secret)?;
                vec![secret.clone().into_bytes()]
            }
            None => Vec::new(),
        };
        Ok((current.into_bytes(), previous))
    }
}

fn check_secret_len(name: &str, secret: &str) -> Result<()> {
    if secret.len() < MIN_SECRET_LEN {
        anyhow::bail!("{name} must be at least {MIN_SECRET_LEN} characters");
    }
    Ok(())
}
