use iced::{Element, Subscription, Task};

use crate::{browser, login};

pub enum App {
    Login(login::State),
    Browser(Box<browser::State>),
}

#[derive(Debug, Clone)]
pub enum Message {
    Login(login::Message),
    Browser(browser::Message),
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        let (state, task) = login::State::restoring();
        (Self::Login(state), task.map(Message::Login))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match (&mut *self, message) {
            (Self::Login(state), Message::Login(message)) => match state.update(message) {
                login::Update::Task(task) => task.map(Message::Login),
                login::Update::SignedIn { client, me } => {
                    let (state, task) = browser::State::open(client, me);
                    *self = Self::Browser(Box::new(state));
                    task.map(Message::Browser)
                }
            },
            (Self::Browser(state), Message::Browser(message)) => match state.update(message) {
                browser::Update::Task(task) => task.map(Message::Browser),
                browser::Update::SignedOut => {
                    *self = Self::Login(login::State::default());
                    Task::none()
                }
            },
            _ => Task::none(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match self {
            Self::Login(_) => login::subscription().map(Message::Login),
            Self::Browser(state) => state.subscription().map(Message::Browser),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self {
            Self::Login(state) => state.view().map(Message::Login),
            Self::Browser(state) => state.view().map(Message::Browser),
        }
    }
}
