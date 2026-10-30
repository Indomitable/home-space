use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result};
use clap::Args;

const MIN_SECRET_LEN: usize = 32;

/// Settings shared by all subcommands, read from flags or environment.
#[derive(Args, Debug)]
pub struct Config {
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,

    #[arg(long, env = "HOMESPACE_JWT_SECRET", hide_env_values = true)]
    pub jwt_secret: String,

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
        if self.jwt_secret.len() < MIN_SECRET_LEN {
            Err(anyhow::anyhow!(
                "HOMESPACE_JWT_SECRET must be at least {MIN_SECRET_LEN} characters"
            ))
            .context("invalid configuration")
        } else {
            Ok(())
        }
    }
}
