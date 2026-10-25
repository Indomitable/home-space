mod config;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use api::AppState;
use auth::{AuthConfig, AuthService};
use db::Db;
use files::{BlobStore, FileService};
use notifications::Hub;

use config::Config;

#[derive(Parser)]
#[command(name = "homespace", about = "Personal file storage server")]
struct Cli {
    #[command(flatten)]
    config: Config,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the HTTP server.
    Serve,
    /// Apply database migrations.
    Migrate,
    /// Manage users.
    #[command(subcommand)]
    User(UserCommand),
}

#[derive(Subcommand)]
enum UserCommand {
    /// Create a user; the password is prompted for.
    Add { username: String },
    /// List users.
    List,
    /// Change a user's password (revokes their sessions); the password is prompted for.
    Passwd { username: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cli = Cli::parse();
    cli.config.validate()?;
    let db = Db::connect(&cli.config.database_url)
        .await
        .context("connecting to database")?;
    db.migrate().await.context("running migrations")?;

    match cli.command {
        Command::Migrate => {
            tracing::info!("migrations applied");
            Ok(())
        }
        Command::User(command) => run_user_command(command, &db, &cli.config).await,
        Command::Serve => serve(db, &cli.config).await,
    }
}

fn auth_service(db: &Db, config: &Config) -> Result<AuthService> {
    let auth_config = AuthConfig {
        jwt_secret: config.jwt_secret.clone().into_bytes(),
        access_ttl: time::Duration::seconds(config.access_ttl_secs),
        refresh_ttl: time::Duration::seconds(config.refresh_ttl_secs),
    };
    AuthService::new(db.clone(), auth_config).context("creating auth service")
}

async fn serve(db: Db, config: &Config) -> Result<()> {
    let store = BlobStore::open(&config.storage_root)
        .await
        .context("opening storage root")?;
    let state = AppState {
        auth: auth_service(&db, config)?,
        files: FileService::new(db.clone(), store, config.max_upload_bytes),
        hub: Hub::new(),
        db,
    };
    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .with_context(|| format!("binding {}", config.bind))?;
    tracing::info!(address = %config.bind, "listening");
    axum::serve(listener, api::router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("serving")
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}

async fn run_user_command(command: UserCommand, db: &Db, config: &Config) -> Result<()> {
    match command {
        UserCommand::Add { username } => {
            let password = prompt_new_password()?;
            let user = auth_service(db, config)?
                .create_user(&username, &password)
                .await
                .context("creating user")?;
            println!("created user {} ({})", user.username, user.id);
        }
        UserCommand::List => {
            for user in db.list_users().await.context("listing users")? {
                println!("{}\t{}", user.id, user.username);
            }
        }
        UserCommand::Passwd { username } => {
            let user = db
                .find_user_by_username(&username)
                .await
                .context("looking up user")?
                .with_context(|| format!("no such user: {username}"))?;
            let password = prompt_new_password()?;
            auth_service(db, config)?
                .set_password(user.id, &password)
                .await
                .context("changing password")?;
            println!("password updated for {username}");
        }
    }
    Ok(())
}

fn prompt_new_password() -> Result<String> {
    let password = rpassword::prompt_password("Password: ").context("reading password")?;
    let confirmation =
        rpassword::prompt_password("Confirm password: ").context("reading password")?;
    if password == confirmation {
        Ok(password)
    } else {
        Err(anyhow::anyhow!("passwords do not match"))
    }
}
