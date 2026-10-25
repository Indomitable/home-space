use iced::{
    Element, Length, Subscription, Task,
    keyboard::{self, Key, key::Named},
    widget::{button, column, container, operation, text, text_input},
};

use api_client::{ApiError, Client};
use protocol::Me;

use crate::session::{self, Restored};

const DEFAULT_SERVER: &str = "http://127.0.0.1:8080";
const FORM_WIDTH: f32 = 360.0;
const FORM_SPACING: f32 = 12.0;
const FORM_PADDING: f32 = 24.0;
const RESUME_FAILED: &str = "Could not resume the session";

#[derive(Debug, Clone)]
pub enum Message {
    ServerChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    Submit,
    FocusNext,
    FocusPrevious,
    Finished(Result<(Client, Me), ApiError>),
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
}

impl Default for State {
    fn default() -> Self {
        Self {
            server: DEFAULT_SERVER.to_owned(),
            username: String::new(),
            password: String::new(),
            busy: false,
            error: None,
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
                let server = self.server.trim().trim_end_matches('/').to_owned();
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
                self.error = Some(err.to_string());
                Update::Task(Task::none())
            }
            Message::Restored(Restored::Nothing) => {
                self.busy = false;
                Update::Task(Task::none())
            }
            Message::Restored(Restored::Failed { server, error }) => {
                self.busy = false;
                self.server = server;
                self.error = match error {
                    ApiError::Unauthorized => None,
                    error => Some(format!("{RESUME_FAILED}: {error}")),
                };
                Update::Task(Task::none())
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let submit = button(if self.busy {
            "Signing in…"
        } else {
            "Sign in"
        })
        .on_press_maybe((!self.busy).then_some(Message::Submit));
        let mut form = column![
            text("Home Space").size(28),
            text_input("Server", &self.server).on_input(Message::ServerChanged),
            text_input("Username", &self.username)
                .on_input(Message::UsernameChanged)
                .on_submit(Message::Submit),
            text_input("Password", &self.password)
                .secure(true)
                .on_input(Message::PasswordChanged)
                .on_submit(Message::Submit),
            submit,
        ]
        .spacing(FORM_SPACING)
        .width(FORM_WIDTH);
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
) -> Result<(Client, Me), ApiError> {
    let client = session::client(&server);
    let me = client.login(&username, &password).await?;
    Ok((client, me))
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
}
