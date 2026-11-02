use std::time::Duration;

use iced::{
    Element, Length, Subscription, Task,
    keyboard::{self, Key, key::Named},
    task,
    widget::{button, column, container, operation, text, text_input},
};

use api_client::{ApiError, Client};
use protocol::{Me, Providers};

use crate::{
    session::{self, Restored},
    sso,
};

const DEFAULT_SERVER: &str = "http://127.0.0.1:8080";
const FORM_WIDTH: f32 = 360.0;
const FORM_SPACING: f32 = 12.0;
const FORM_PADDING: f32 = 24.0;
const RESUME_FAILED: &str = "Could not resume the session";
/// How long the server field must rest before the server is asked how users sign in.
const PROBE_DELAY: Duration = Duration::from_millis(600);

#[derive(Debug, Clone)]
pub enum Message {
    ServerChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    Submit,
    SignInWithProvider,
    Cancel,
    /// The server field has rested for [`PROBE_DELAY`]; carries the probe generation and server.
    Probe(u64, String),
    ProvidersLoaded(u64, Result<Providers, String>),
    FocusNext,
    FocusPrevious,
    Finished(Result<(Client, Me), String>),
    Restored(Restored),
}

pub enum Update {
    Task(Task<Message>),
    SignedIn { client: Client, me: Me },
}

pub struct State {
    server: String,
    username: String,
    password: String,
    busy: bool,
    error: Option<String>,
    /// How the server lets users sign in, once known; the password form shows until then.
    providers: Option<Providers>,
    /// Bumped whenever the server changes, so answers about an earlier one are ignored.
    probe: u64,
    /// The browser sign-in in progress, if any.
    sso: Option<task::Handle>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            server: DEFAULT_SERVER.to_owned(),
            username: String::new(),
            password: String::new(),
            busy: false,
            error: None,
            providers: None,
            probe: 0,
            sso: None,
        }
    }
}

impl State {
    /// The login screen, busy while it resumes the session saved by an earlier run.
    pub fn restoring() -> (Self, Task<Message>) {
        let state = Self {
            busy: true,
            ..Self::default()
        };
        (state, Task::perform(session::restore(), Message::Restored))
    }

    pub fn update(&mut self, message: Message) -> Update {
        match message {
            Message::ServerChanged(value) => {
                self.server = value;
                Update::Task(self.schedule_probe())
            }
            Message::Probe(probe, server) if probe == self.probe && !server.is_empty() => {
                Update::Task(Task::perform(fetch_providers(server), move |result| {
                    Message::ProvidersLoaded(probe, result)
                }))
            }
            Message::Probe(..) => Update::Task(Task::none()),
            Message::ProvidersLoaded(probe, result) => {
                // An unreachable server leaves the password form, which reports the problem.
                if probe == self.probe {
                    self.providers = result.ok();
                }
                Update::Task(Task::none())
            }
            Message::SignInWithProvider if self.busy => Update::Task(Task::none()),
            Message::SignInWithProvider => {
                self.busy = true;
                self.error = None;
                let (task, handle) =
                    Task::perform(sso::sign_in(self.server_url()), Message::Finished).abortable();
                self.sso = Some(handle);
                Update::Task(task)
            }
            Message::Cancel => {
                if let Some(handle) = self.sso.take() {
                    handle.abort();
                }
                self.busy = false;
                Update::Task(Task::none())
            }
            Message::UsernameChanged(value) => {
                self.username = value;
                Update::Task(Task::none())
            }
            Message::PasswordChanged(value) => {
                self.password = value;
                Update::Task(Task::none())
            }
            Message::FocusNext => Update::Task(operation::focus_next()),
            Message::FocusPrevious => Update::Task(operation::focus_previous()),
            Message::Submit if self.busy => Update::Task(Task::none()),
            Message::Submit => {
                self.busy = true;
                self.error = None;
                let server = self.server_url();
                let username = self.username.trim().to_owned();
                let password = self.password.clone();
                Update::Task(Task::perform(
                    sign_in(server, username, password),
                    Message::Finished,
                ))
            }
            Message::Finished(Ok((client, me)))
            | Message::Restored(Restored::SignedIn { client, me }) => {
                Update::SignedIn { client, me }
            }
            Message::Finished(Err(err)) => {
                self.busy = false;
                self.sso = None;
                self.error = Some(err);
                Update::Task(Task::none())
            }
            Message::Restored(Restored::Nothing) => {
                self.busy = false;
                Update::Task(self.schedule_probe())
            }
            Message::Restored(Restored::Failed { server, error }) => {
                self.busy = false;
                self.server = server;
                self.error = match error {
                    ApiError::Unauthorized => None,
                    error => Some(format!("{RESUME_FAILED}: {error}")),
                };
                Update::Task(self.schedule_probe())
            }
        }
    }

    fn server_url(&self) -> String {
        self.server.trim().trim_end_matches('/').to_owned()
    }

    /// Asks the server how users sign in once the server field stops changing.
    fn schedule_probe(&mut self) -> Task<Message> {
        self.probe += 1;
        self.providers = None;
        let (probe, server) = (self.probe, self.server_url());
        Task::perform(
            async move {
                tokio::time::sleep(PROBE_DELAY).await;
                (probe, server)
            },
            |(probe, server)| Message::Probe(probe, server),
        )
    }

    pub fn view(&self) -> Element<'_, Message> {
        let mut form = column![
            text("Home Space").size(28),
            text_input("Server", &self.server).on_input(Message::ServerChanged),
        ]
        .spacing(FORM_SPACING)
        .width(FORM_WIDTH);
        let provider = self
            .providers
            .as_ref()
            .and_then(|providers| providers.oidc.as_ref());
        if let Some(provider) = provider {
            let label = if self.busy {
                "Waiting for the browser…".to_owned()
            } else {
                format!("Sign in with {}", provider.name)
            };
            form = form.push(
                button(text(label))
                    .on_press_maybe((!self.busy).then_some(Message::SignInWithProvider)),
            );
            if self.sso.is_some() {
                form = form.push(button("Cancel").on_press(Message::Cancel));
            }
        }
        let password_login = self
            .providers
            .as_ref()
            .is_none_or(|providers| providers.password);
        if password_login {
            if provider.is_some() {
                form = form.push(text("or"));
            }
            form = form
                .push(
                    text_input("Username", &self.username)
                        .on_input(Message::UsernameChanged)
                        .on_submit(Message::Submit),
                )
                .push(
                    text_input("Password", &self.password)
                        .secure(true)
                        .on_input(Message::PasswordChanged)
                        .on_submit(Message::Submit),
                )
                .push(
                    button(if self.busy {
                        "Signing in…"
                    } else {
                        "Sign in"
                    })
                    .on_press_maybe((!self.busy).then_some(Message::Submit)),
                );
        }
        if let Some(error) = &self.error {
            form = form.push(text(error.as_str()));
        }
        container(form.padding(FORM_PADDING))
            .center(Length::Fill)
            .into()
    }
}

pub fn subscription() -> Subscription<Message> {
    keyboard::listen().filter_map(|event| match event {
        keyboard::Event::KeyPressed {
            key: Key::Named(Named::Tab),
            modifiers,
            ..
        } if modifiers.shift() => Some(Message::FocusPrevious),
        keyboard::Event::KeyPressed {
            key: Key::Named(Named::Tab),
            ..
        } => Some(Message::FocusNext),
        _ => None,
    })
}

async fn sign_in(
    server: String,
    username: String,
    password: String,
) -> Result<(Client, Me), String> {
    let client = session::client(&server);
    let me = client
        .login(&username, &password)
        .await
        .map_err(|err| err.to_string())?;
    Ok((client, me))
}

async fn fetch_providers(server: String) -> Result<Providers, String> {
    Client::new(server)
        .providers()
        .await
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAVED_SERVER: &str = "http://nas.local:8080";

    #[test]
    fn an_expired_saved_session_shows_the_form_for_its_server() {
        let mut state = State::default();

        let _ = state.update(Message::Restored(Restored::Failed {
            server: SAVED_SERVER.to_owned(),
            error: ApiError::Unauthorized,
        }));

        assert_eq!(
            (state.server.as_str(), state.busy, state.error),
            (SAVED_SERVER, false, None)
        );
    }

    fn providers(password: bool) -> Providers {
        Providers {
            password,
            oidc: Some(protocol::OidcProvider {
                name: "Keycloak".to_owned(),
            }),
        }
    }

    #[test]
    fn providers_for_the_current_server_are_kept() {
        let mut state = State::default();
        let _ = state.update(Message::ServerChanged("http://nas.local".to_owned()));

        let _ = state.update(Message::ProvidersLoaded(state.probe, Ok(providers(false))));

        assert_eq!(state.providers, Some(providers(false)));
    }

    #[test]
    fn providers_for_an_earlier_server_are_ignored() {
        let mut state = State::default();
        let _ = state.update(Message::ServerChanged("http://old.local".to_owned()));
        let stale = state.probe;
        let _ = state.update(Message::ServerChanged("http://new.local".to_owned()));

        let _ = state.update(Message::ProvidersLoaded(stale, Ok(providers(true))));

        assert_eq!(state.providers, None);
    }

    #[test]
    fn changing_the_server_forgets_its_providers() {
        let mut state = State::default();
        let _ = state.update(Message::ProvidersLoaded(state.probe, Ok(providers(true))));

        let _ = state.update(Message::ServerChanged("http://other.local".to_owned()));

        assert_eq!(state.providers, None);
    }

    #[test]
    fn a_failed_browser_sign_in_shows_the_error_and_frees_the_form() {
        let mut state = State::default();
        let _ = state.update(Message::SignInWithProvider);
        assert!(state.busy);

        let _ = state.update(Message::Finished(Err("denied".to_owned())));

        assert_eq!(
            (state.busy, state.sso.is_none(), state.error.as_deref()),
            (false, true, Some("denied"))
        );
    }

    #[test]
    fn cancelling_a_browser_sign_in_frees_the_form() {
        let mut state = State::default();
        let _ = state.update(Message::SignInWithProvider);

        let _ = state.update(Message::Cancel);

        assert_eq!((state.busy, state.sso.is_none()), (false, true));
    }
}
