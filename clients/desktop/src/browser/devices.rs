use iced::{
    Alignment, Element, Length, Padding, Task,
    widget::{button, column, container, operation, row, scrollable, text, text_input},
};
use uuid::Uuid;

use api_client::{ApiError, Client};
use client_core::labels::{THIS_DEVICE, device_state, platform, sign_out_question};
use protocol::Device;

use super::grid::{
    CELL_SPACING, DATE_WIDTH, EMPTY_PADDING, HEADER_SIZE, ROW_GAP, ROW_PADDING, ROW_SIZE,
    SCROLLBAR_GUTTER,
};
use crate::{
    ellipsis::ellipsized,
    icons::{ICON_SIZE, Icon, Tint},
    locale::short_date,
    theme,
};

const SPACING: f32 = 4.0;
const LABEL_SPACING: f32 = 6.0;
const NOTE_SIZE: f32 = 13.0;
const NAME_PORTION: u16 = 3;
const STATE_PORTION: u16 = 2;
const ACTIONS_WIDTH: f32 = 220.0;
const NAME_INPUT: &str = "device-name";
const SIGN_OUT: &str = "Sign out";

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(Result<Vec<Device>, ApiError>),
    Refresh,
    /// Starts renaming a device.
    Rename(Uuid),
    NameChanged(String),
    Save,
    Renamed(Result<(), ApiError>),
    /// Asks before signing a device out.
    SignOut(Uuid),
    ConfirmSignOut,
    SignedOut(Result<(), ApiError>),
    /// Leaves the rename or the sign-out question.
    Cancel,
}

/// A device being renamed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Renaming {
    id: Uuid,
    name: String,
    saving: bool,
}

/// The user's devices, with renaming and signing out.
#[derive(Default)]
pub struct State {
    devices: Vec<Device>,
    loading: bool,
    error: Option<String>,
    renaming: Option<Renaming>,
    /// The device waiting for the user to confirm its sign-out.
    confirming: Option<Uuid>,
}

impl State {
    /// A device list that is still waiting for its first listing.
    pub fn open() -> Self {
        Self {
            loading: true,
            ..Self::default()
        }
    }

    pub fn load(&mut self, client: &Client) -> Task<Message> {
        self.loading = true;
        let client = client.clone();
        Task::perform(async move { client.devices().await }, Message::Loaded)
    }

    /// Whether a rename or a sign-out question is open, which Escape leaves first.
    pub fn is_asking(&self) -> bool {
        self.renaming.is_some() || self.confirming.is_some()
    }

    pub fn update(&mut self, client: &Client, message: Message) -> Task<Message> {
        match message {
            Message::Loaded(result) => {
                self.loading = false;
                match result {
                    Ok(devices) => {
                        self.devices = devices;
                        self.error = None;
                    }
                    Err(err) => self.error = Some(err.to_string()),
                }
                Task::none()
            }
            Message::Refresh => self.load(client),
            Message::Rename(id) => {
                let Some(name) = self.device(id).map(|device| device.name.clone()) else {
                    return Task::none();
                };
                self.confirming = None;
                self.renaming = Some(Renaming {
                    id,
                    name,
                    saving: false,
                });
                Task::batch([
                    operation::focus(NAME_INPUT),
                    operation::select_all(NAME_INPUT),
                ])
            }
            Message::NameChanged(name) => {
                if let Some(renaming) = &mut self.renaming {
                    renaming.name = name;
                }
                Task::none()
            }
            Message::Save => self.save(client),
            Message::Renamed(result) => {
                self.renaming = None;
                self.done(client, result)
            }
            Message::SignOut(id) => {
                self.renaming = None;
                self.confirming = Some(id);
                Task::none()
            }
            Message::ConfirmSignOut => {
                let Some(id) = self.confirming.take() else {
                    return Task::none();
                };
                let client = client.clone();
                Task::perform(
                    async move { client.sign_out_device(id).await },
                    Message::SignedOut,
                )
            }
            Message::SignedOut(result) => self.done(client, result),
            Message::Cancel => {
                self.renaming = None;
                self.confirming = None;
                Task::none()
            }
        }
    }

    fn device(&self, id: Uuid) -> Option<&Device> {
        self.devices.iter().find(|device| device.id == id)
    }

    /// Sends the new name, unless it is blank or unchanged.
    fn save(&mut self, client: &Client) -> Task<Message> {
        let Some(renaming) = &mut self.renaming else {
            return Task::none();
        };
        let name = renaming.name.trim().to_owned();
        let unchanged = self
            .devices
            .iter()
            .any(|device| device.id == renaming.id && device.name == name);
        if renaming.saving || name.is_empty() {
            Task::none()
        } else if unchanged {
            self.renaming = None;
            Task::none()
        } else {
            renaming.saving = true;
            let (client, id) = (client.clone(), renaming.id);
            Task::perform(
                async move { client.rename_device(id, &name).await },
                Message::Renamed,
            )
        }
    }

    /// Shows what went wrong, if anything, and reloads the list either way.
    fn done(&mut self, client: &Client, result: Result<(), ApiError>) -> Task<Message> {
        if let Err(err) = result {
            self.error = Some(err.to_string());
        }
        self.load(client)
    }

    pub fn view(&self) -> Element<'_, Message> {
        let refresh = tool(Icon::Refresh, "Refresh", Some(Message::Refresh), false);
        let status = text(if self.loading { "Loading…" } else { "" });
        let bar = row![refresh, status]
            .spacing(SPACING)
            .align_y(Alignment::Center);
        let mut page = column![bar].spacing(SPACING * 3.0);
        if let Some(error) = &self.error {
            page = page.push(container(text(error.as_str())).style(theme::error));
        }
        page.push(
            column![header(), self.rows()]
                .spacing(SPACING)
                .height(Length::Fill),
        )
        .into()
    }

    fn rows(&self) -> Element<'_, Message> {
        if self.devices.is_empty() {
            let note = if self.loading {
                ""
            } else {
                "No devices have signed in yet."
            };
            return container(text(note))
                .padding(EMPTY_PADDING)
                .center_x(Length::Fill)
                .into();
        }
        let rows = self.devices.iter().map(|device| self.device_row(device));
        scrollable(
            column(rows)
                .spacing(ROW_GAP)
                .padding(Padding::default().right(SCROLLBAR_GUTTER)),
        )
        .height(Length::Fill)
        .into()
    }

    fn device_row<'a>(&'a self, device: &'a Device) -> Element<'a, Message> {
        let cells = match (&self.renaming, self.confirming) {
            (Some(renaming), _) if renaming.id == device.id => rename_cells(renaming),
            (_, Some(id)) if id == device.id => confirm_cells(device),
            _ => device_cells(device),
        };
        container(cells.spacing(CELL_SPACING).align_y(Alignment::Center))
            .padding(ROW_PADDING)
            .into()
    }
}

fn header<'a>() -> Element<'a, Message> {
    let title = |label: &'static str| text(label).size(HEADER_SIZE);
    let cells = row![
        title("Name").width(Length::FillPortion(NAME_PORTION)),
        title("Last seen").width(DATE_WIDTH),
        title("State").width(Length::FillPortion(STATE_PORTION)),
        title("").width(ACTIONS_WIDTH),
    ]
    .spacing(CELL_SPACING)
    .align_y(Alignment::Center);
    container(container(cells).padding(ROW_PADDING).style(theme::header))
        .padding(Padding::default().right(SCROLLBAR_GUTTER))
        .into()
}

fn device_cells(device: &Device) -> iced::widget::Row<'_, Message> {
    let kind = if device.current {
        format!("{} · {THIS_DEVICE}", platform(&device.platform))
    } else {
        platform(&device.platform).to_owned()
    };
    let name = row![
        Icon::Devices.view(ICON_SIZE, Tint::Muted),
        column![
            ellipsized(device.name.as_str(), ROW_SIZE, Alignment::Start),
            container(text(kind).size(NOTE_SIZE)).style(theme::muted),
        ]
        .width(Length::Fill)
    ]
    .spacing(CELL_SPACING / 1.5)
    .align_y(Alignment::Center)
    .width(Length::FillPortion(NAME_PORTION));
    // The current device signs out with the header's Sign out; one that is signed out
    // and left nothing behind has nothing to end.
    let can_sign_out = !device.current && (device.signed_in || device.uploads > 0);
    let actions = row![
        tool(
            Icon::Rename,
            "Rename",
            Some(Message::Rename(device.id)),
            false
        ),
        tool(
            Icon::SignOut,
            SIGN_OUT,
            can_sign_out.then_some(Message::SignOut(device.id)),
            true
        ),
    ]
    .spacing(SPACING)
    .width(ACTIONS_WIDTH);
    row![
        name,
        ellipsized(short_date(device.last_seen_at), ROW_SIZE, Alignment::Start).width(DATE_WIDTH),
        ellipsized(device_state(device), ROW_SIZE, Alignment::Start)
            .width(Length::FillPortion(STATE_PORTION)),
        actions,
    ]
}

fn rename_cells(renaming: &Renaming) -> iced::widget::Row<'_, Message> {
    let can_save = !renaming.saving && !renaming.name.trim().is_empty();
    row![
        Icon::Devices.view(ICON_SIZE, Tint::Muted),
        text_input("Device name", &renaming.name)
            .id(NAME_INPUT)
            .on_input_maybe((!renaming.saving).then_some(Message::NameChanged))
            .on_submit(Message::Save)
            .width(Length::Fill),
        button(text("Save")).on_press_maybe(can_save.then_some(Message::Save)),
        button(text("Cancel"))
            .style(button::secondary)
            .on_press(Message::Cancel),
    ]
}

fn confirm_cells(device: &Device) -> iced::widget::Row<'_, Message> {
    row![
        Icon::SignOut.view(ICON_SIZE, Tint::Danger),
        text(sign_out_question(device)).width(Length::Fill),
        button(text("Cancel"))
            .style(button::secondary)
            .on_press(Message::Cancel),
        button(text(SIGN_OUT))
            .style(button::danger)
            .on_press(Message::ConfirmSignOut),
    ]
}

fn tool(icon: Icon, label: &str, message: Option<Message>, danger: bool) -> Element<'_, Message> {
    let (style, tint): (fn(&iced::Theme, button::Status) -> button::Style, Tint) = if danger {
        (theme::danger_tool, Tint::Danger)
    } else {
        (theme::tool, Tint::Text)
    };
    let tint = if message.is_some() {
        tint
    } else {
        Tint::Disabled
    };
    button(
        row![icon.view(ICON_SIZE, tint), text(label)]
            .spacing(LABEL_SPACING)
            .align_y(Alignment::Center),
    )
    .style(style)
    .on_press_maybe(message)
    .into()
}

#[cfg(test)]
mod tests {
    use iced_test::simulator;
    use time::OffsetDateTime;

    use protocol::{PLATFORM_LINUX, PLATFORM_WEB};

    use super::*;

    const LAPTOP: u128 = 1;
    const BROWSER: u128 = 2;

    fn device(raw: u128, name: &str, platform: &str, current: bool) -> Device {
        Device {
            id: Uuid::from_u128(raw),
            name: name.to_owned(),
            platform: platform.to_owned(),
            created_at: OffsetDateTime::UNIX_EPOCH,
            last_seen_at: OffsetDateTime::UNIX_EPOCH,
            signed_in: true,
            online: false,
            current,
            uploads: 0,
        }
    }

    fn client() -> Client {
        Client::new("http://localhost")
    }

    fn loaded() -> State {
        let mut state = State::open();
        let devices = vec![
            device(LAPTOP, "Laptop", PLATFORM_LINUX, true),
            Device {
                uploads: 2,
                ..device(BROWSER, "Firefox", PLATFORM_WEB, false)
            },
        ];
        let _ = state.update(&client(), Message::Loaded(Ok(devices)));
        state
    }

    #[test]
    fn the_list_names_each_device_and_marks_this_one() {
        let state = loaded();
        let mut ui = simulator(state.view());

        assert!(ui.find("Laptop").is_ok());
        assert!(ui.find("Linux · This device").is_ok());
        assert!(ui.find("Web browser").is_ok());
        assert!(ui.find("Signed in · 2 unfinished uploads").is_ok());
    }

    #[test]
    fn this_device_cannot_be_signed_out_from_the_list() {
        let state = State {
            devices: vec![device(LAPTOP, "Laptop", PLATFORM_LINUX, true)],
            ..State::default()
        };
        let mut ui = simulator(state.view());

        ui.click(SIGN_OUT).expect("the button is shown");

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn signing_out_asks_first_and_names_what_it_cancels() {
        let mut state = loaded();

        let _ = state.update(&client(), Message::SignOut(Uuid::from_u128(BROWSER)));
        let mut ui = simulator(state.view());

        assert!(
            ui.find("Sign out “Firefox” and cancel its 2 unfinished uploads?")
                .is_ok()
        );
    }

    #[test]
    fn confirming_sends_the_sign_out_and_closes_the_question() {
        let mut state = loaded();
        let _ = state.update(&client(), Message::SignOut(Uuid::from_u128(BROWSER)));

        let task = state.update(&client(), Message::ConfirmSignOut);

        assert!(task.units() > 0);
        assert!(!state.is_asking());
    }

    #[test]
    fn renaming_starts_from_the_current_name() {
        let mut state = loaded();

        let _ = state.update(&client(), Message::Rename(Uuid::from_u128(LAPTOP)));

        assert_eq!(
            state
                .renaming
                .as_ref()
                .map(|renaming| renaming.name.as_str()),
            Some("Laptop")
        );
    }

    #[test]
    fn a_blank_name_is_not_saved() {
        let mut state = loaded();
        let _ = state.update(&client(), Message::Rename(Uuid::from_u128(LAPTOP)));
        let _ = state.update(&client(), Message::NameChanged("  ".to_owned()));

        let task = state.update(&client(), Message::Save);

        assert_eq!(task.units(), 0);
        assert!(state.renaming.as_ref().is_some_and(|r| !r.saving));
    }

    #[test]
    fn a_new_name_is_sent() {
        let mut state = loaded();
        let _ = state.update(&client(), Message::Rename(Uuid::from_u128(LAPTOP)));
        let _ = state.update(&client(), Message::NameChanged(" Work laptop ".to_owned()));

        let task = state.update(&client(), Message::Save);

        assert!(task.units() > 0);
        assert!(state.renaming.as_ref().is_some_and(|r| r.saving));
    }

    #[test]
    fn a_failed_sign_out_is_shown() {
        let mut state = loaded();

        let _ = state.update(&client(), Message::SignedOut(Err(ApiError::Unauthorized)));

        assert!(state.error.is_some());
    }
}
