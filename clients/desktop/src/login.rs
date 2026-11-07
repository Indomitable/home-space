use std::time::Duration;

use api_client::{ApiError, Client};
use iced::widget::{row, space};
use iced::{
    Alignment, Element, Length, Padding, Subscription, Task,
    keyboard::{self, Key, key::Named},
    task,
    widget::{button, column, container, operation, text, text_input},
};
use protocol::{Me, Providers};

use crate::{
    session::{self, Restored, Saved},
    sso, theme,
};

const DEFAULT_SERVER: &str = "http://127.0.0.1:8080";
const FORM_WIDTH: f32 = 360.0;
const FORM_SPACING: f32 = 12.0;
const FORM_PADDING: f32 = 24.0;
const INPUT_PADDING: Padding = Padding {
    top: 8.0,
    left: 8.0,
    bottom: 8.0,
    right: 8.0,
};
const BUTTON_HEIGHT: Length = Length::Fixed(40.0);
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
    /// The session saved by an earlier run has been read, before it is resumed.
    SavedLoaded(Option<Saved>),
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
    /// The session saved by an earlier run is being resumed.
    restoring: bool,
    error: Option<String>,
    /// How the server lets users sign in, once known.
    providers: Option<Providers>,
    /// The server has not answered the first question about how users sign in, so the
    /// password form stays hidden rather than flashing before the provider button replaces it.
    first_probe_pending: bool,
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
            restoring: false,
            error: None,
            providers: None,
            first_probe_pending: true,
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
            restoring: true,
            ..Self::default()
        };
        (
            state,
            Task::perform(session::load_saved(), Message::SavedLoaded),
        )
    }

    /// The login screen after signing out of `server`, which it offers again.
    pub fn signed_out(server: String) -> (Self, Task<Message>) {
        let mut state = Self {
            server,
            ..Self::default()
        };
        let task = state.schedule_probe();
        (state, task)
    }

    pub fn update(&mut self, message: Message) -> Update {
        match message {
            Message::ServerChanged(_) if self.restoring => Update::Task(Task::none()),
            Message::ServerChanged(value) => {
                self.server = value;
                Update::Task(self.schedule_probe())
            }
            Message::Probe(probe, server) if probe == self.probe && !server.is_empty() => {
                Update::Task(Task::perform(fetch_providers(server), move |result| {
                    Message::ProvidersLoaded(probe, result)
                }))
            }
            Message::Probe(probe, _) => {
                if probe == self.probe {
                    self.first_probe_pending = false;
                }
                Update::Task(Task::none())
            }
            Message::ProvidersLoaded(probe, result) => {
                // An unreachable server leaves the password form, which reports the problem.
                if probe == self.probe {
                    self.providers = result.ok();
                    self.first_probe_pending = false;
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
            Message::SavedLoaded(Some(saved)) => {
                self.server.clone_from(&saved.server);
                Update::Task(Task::perform(session::resume(saved), Message::Restored))
            }
            Message::SavedLoaded(None) => {
                self.busy = false;
                self.restoring = false;
                Update::Task(self.schedule_probe())
            }
            Message::Restored(Restored::Failed { server, error }) => {
                self.busy = false;
                self.restoring = false;
                self.server = server;
                self.error = match error {
                    ApiError::Unauthorized => None,
                    error => Some(format!("{RESUME_FAILED}: {error}")),
                };
                Update::Task(self.schedule_probe())
            }
        }
    }

    /// What the screen is waiting for while it has no buttons to offer.
    fn status(&self) -> Option<&'static str> {
        if self.restoring {
            Some("Resuming your session…")
        } else if self.first_probe_pending && self.providers.is_none() {
            Some("Contacting the server…")
        } else {
            None
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
            container("Home Space")
                .align_x(Alignment::Center)
                .width(Length::Fill),
            container("Sign in to your files")
                .style(theme::muted)
                .align_x(Alignment::Center)
                .width(Length::Fill),
            text_input("Server", &self.server)
                .style(theme::input)
                .padding(INPUT_PADDING)
                .on_input_maybe((!self.restoring).then_some(Message::ServerChanged)),
        ]
        .spacing(FORM_SPACING)
        .width(FORM_WIDTH);
        if let Some(status) = self.status() {
            form = form.push(
                container(text(status))
                    .style(theme::muted)
                    .align_x(Alignment::Center)
                    .width(Length::Fill),
            );
        }
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
            form = form.push(row![
                button(
                    text(label)
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                )
                .style(theme::primary)
                .on_press_maybe((!self.busy).then_some(Message::SignInWithProvider))
                .width(Length::Fill)
                .height(BUTTON_HEIGHT)
            ]);
            if self.sso.is_some() {
                form = form.push(
                    button("Cancel")
                        .style(theme::secondary)
                        .on_press(Message::Cancel),
                );
            }
        }
        let password_login = !self.first_probe_pending
            && self
                .providers
                .as_ref()
                .is_none_or(|providers| providers.password);
        if password_login {
            if provider.is_some() {
                form = form.push(
                    row![
                        container(space())
                            .width(Length::FillPortion(2))
                            .style(theme::divider)
                            .height(Length::Fixed(1.0)),
                        text("or")
                            .width(Length::FillPortion(1))
                            .align_x(text::Alignment::Center),
                        container(space())
                            .width(Length::FillPortion(2))
                            .style(theme::divider)
                            .height(Length::Fixed(1.0))
                    ]
                    .align_y(Alignment::Center),
                );
            }
            form = form
                .push(
                    text_input("Username", &self.username)
                        .style(theme::input)
                        .on_input(Message::UsernameChanged)
                        .on_submit(Message::Submit)
                        .padding(INPUT_PADDING),
                )
                .push(
                    text_input("Password", &self.password)
                        .style(theme::input)
                        .secure(true)
                        .on_input(Message::PasswordChanged)
                        .on_submit(Message::Submit)
                        .padding(INPUT_PADDING),
                )
                .push(row![
                    button(
                        text(if self.busy {
                            "Signing in…"
                        } else {
                            "Sign in"
                        })
                        .align_x(Alignment::Center)
                        .align_y(Alignment::Center)
                    )
                    .style(if provider.is_some() {
                        theme::secondary
                    } else {
                        theme::primary
                    })
                    .on_press_maybe((!self.busy).then_some(Message::Submit))
                    .height(BUTTON_HEIGHT)
                    .width(Length::Fill)
                ]);
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
    fn the_login_screen_after_signing_out_asks_the_server_how_to_sign_in() {
        let (state, _task) = State::signed_out(SAVED_SERVER.to_owned());

        assert_eq!(state.probe, 1);
        assert!(state.first_probe_pending);
        assert_eq!(state.server, SAVED_SERVER);
    }

    #[test]
    fn the_password_form_waits_for_the_first_answer_and_returns_if_there_is_none() {
        let mut state = State::default();
        let _ = state.update(Message::ServerChanged("http://nas.local".to_owned()));
        assert!(state.first_probe_pending);

        let _ = state.update(Message::ProvidersLoaded(
            state.probe,
            Err("down".to_owned()),
        ));

        assert!(!state.first_probe_pending);
        assert_eq!(state.providers, None);
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
    fn a_saved_session_shows_its_server_while_it_resumes() {
        let (mut state, _task) = State::restoring();

        let _ = state.update(Message::SavedLoaded(Some(Saved {
            server: SAVED_SERVER.to_owned(),
            refresh_token: "token".to_owned(),
        })));

        assert_eq!(
            (state.server.as_str(), state.restoring),
            (SAVED_SERVER, true)
        );
    }

    #[test]
    fn the_server_field_is_editable_once_resuming_fails() {
        let (mut state, _task) = State::restoring();
        let _ = state.update(Message::ServerChanged("http://typed.local".to_owned()));
        assert_eq!(state.server, DEFAULT_SERVER);

        let _ = state.update(Message::Restored(Restored::Failed {
            server: SAVED_SERVER.to_owned(),
            error: ApiError::Unauthorized,
        }));
        let _ = state.update(Message::ServerChanged("http://typed.local".to_owned()));

        assert_eq!(state.server, "http://typed.local");
    }

    #[test]
    fn resuming_a_session_says_so_until_it_finishes() {
        let (mut state, _task) = State::restoring();
        assert_eq!(state.status(), Some("Resuming your session…"));

        let _ = state.update(Message::SavedLoaded(None));

        assert_eq!(state.status(), Some("Contacting the server…"));
        let _ = state.update(Message::ProvidersLoaded(state.probe, Ok(providers(false))));
        assert_eq!(state.status(), None);
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
