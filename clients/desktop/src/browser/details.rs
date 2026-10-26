use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, scrollable, space, text},
};

use api_client::{ApiError, Client, format::human_size};
use protocol::{FileVersion, Node, NodeId, NodeKind};

use super::Message as BrowserMessage;
use crate::{
    icons::{ICON_SIZE, Icon, Tint},
    locale::short_date,
    theme,
};

const WIDTH: f32 = 320.0;
const PADDING: f32 = 16.0;
const SPACING: f32 = 12.0;
const ROW_SPACING: f32 = 8.0;
const TITLE_SIZE: f32 = 18.0;
const META_SIZE: f32 = 13.0;
const LABEL_WIDTH: f32 = 72.0;
const FOLDER_LABEL: &str = "Folder";

#[derive(Debug, Clone)]
pub enum Message {
    Loaded(NodeId, Result<(Node, Vec<FileVersion>), ApiError>),
    Download(i32),
    Downloaded(Result<(), String>),
    AskDelete(i32),
    CancelDelete,
    Delete(i32),
    Deleted(NodeId, Result<(), ApiError>),
    Close,
}

/// The details panel of one node.
pub struct State {
    node: Node,
    versions: Vec<FileVersion>,
    confirming: Option<i32>,
    error: Option<String>,
}

impl State {
    /// Opens the panel for `node` and loads its versions.
    pub fn open(client: &Client, node: Node) -> (Self, Task) {
        let task = load(client, node.id);
        let state = Self {
            node,
            versions: Vec::new(),
            confirming: None,
            error: None,
        };
        (state, task)
    }

    pub fn node_id(&self) -> NodeId {
        self.node.id
    }

    pub fn refresh(&self, client: &Client) -> Task {
        load(client, self.node.id)
    }

    /// Applies `message`; `None` means the panel should close.
    pub fn update(&mut self, client: &Client, message: Message) -> Option<Task> {
        match message {
            Message::Loaded(id, result) if id == self.node.id => {
                match result {
                    Ok((node, versions)) => {
                        self.node = node;
                        self.versions = versions;
                        self.error = None;
                    }
                    Err(err) => self.error = Some(err.to_string()),
                }
                Some(Task::none())
            }
            Message::Loaded(..) => Some(Task::none()),
            Message::Download(version) => {
                Some(download(client.clone(), self.node.clone(), version))
            }
            Message::Downloaded(result) => {
                self.error = result.err();
                Some(Task::none())
            }
            Message::AskDelete(version) => {
                self.confirming = Some(version);
                Some(Task::none())
            }
            Message::CancelDelete => {
                self.confirming = None;
                Some(Task::none())
            }
            Message::Delete(version) => {
                self.confirming = None;
                let (client, id) = (client.clone(), self.node.id);
                Some(Task::perform(
                    async move { client.delete_version(id, version).await },
                    move |result| Message::Deleted(id, result),
                ))
            }
            Message::Deleted(id, result) if id == self.node.id => {
                self.error = result.err().map(|err| err.to_string());
                Some(load(client, id))
            }
            Message::Deleted(..) => Some(Task::none()),
            Message::Close => None,
        }
    }

    pub fn view(&self) -> Element<'_, BrowserMessage> {
        let node = &self.node;
        let is_file = node.kind == NodeKind::File;
        let header = row![
            text(node.name.as_str())
                .size(TITLE_SIZE)
                .width(Length::Fill),
            button(Icon::Close.view(ICON_SIZE, Tint::Text))
                .style(theme::tool)
                .on_press(Message::Close),
        ]
        .spacing(ROW_SPACING)
        .align_y(Alignment::Center);

        let kind = if is_file {
            node.mime.as_str()
        } else {
            FOLDER_LABEL
        };
        let mut facts = column![fact("Type", kind.to_owned())].spacing(ROW_SPACING);
        if is_file {
            facts = facts
                .push(fact("Size", human_size(node.size)))
                .push(fact("Version", node.version.to_string()));
        }
        facts = facts
            .push(fact("Created", short_date(node.created_at)))
            .push(fact("Modified", short_date(node.updated_at)));

        let mut panel = column![header].spacing(SPACING);
        if let Some(error) = &self.error {
            panel = panel.push(container(text(error.as_str())).style(theme::error));
        }
        panel = panel.push(facts);
        if is_file {
            panel = panel.push(text("Previous versions").size(TITLE_SIZE - 3.0));
            panel = panel.push(self.version_list());
        }
        let panel: Element<'_, Message> = container(panel)
            .padding(PADDING)
            .width(WIDTH)
            .height(Length::Fill)
            .style(theme::panel)
            .into();
        panel.map(BrowserMessage::Details)
    }

    fn version_list(&self) -> Element<'_, Message> {
        if self.versions.is_empty() {
            return text("No previous versions.").size(META_SIZE).into();
        }
        let rows = self.versions.iter().map(|version| {
            let number = version.version;
            let info = column![
                text(format!("Version {number}")),
                text(format!(
                    "{} · {}",
                    human_size(version.size),
                    short_date(version.modified_at)
                ))
                .size(META_SIZE),
            ];
            let actions: Element<'_, Message> = if self.confirming == Some(number) {
                row![
                    button("Delete")
                        .style(button::danger)
                        .on_press(Message::Delete(number)),
                    button("Cancel")
                        .style(button::secondary)
                        .on_press(Message::CancelDelete),
                ]
                .spacing(ROW_SPACING)
                .into()
            } else {
                row![
                    button(Icon::Download.view(ICON_SIZE, Tint::Text))
                        .style(theme::tool)
                        .on_press(Message::Download(number)),
                    button(Icon::Delete.view(ICON_SIZE, Tint::Danger))
                        .style(theme::danger_tool)
                        .on_press(Message::AskDelete(number)),
                ]
                .into()
            };
            row![info, space::horizontal(), actions]
                .spacing(ROW_SPACING)
                .align_y(Alignment::Center)
                .into()
        });
        scrollable(column(rows).spacing(ROW_SPACING)).into()
    }
}

type Task = iced::Task<Message>;

fn fact<'a>(label: &'a str, value: String) -> Element<'a, Message> {
    row![text(label).width(LABEL_WIDTH).size(META_SIZE), text(value)]
        .spacing(ROW_SPACING)
        .into()
}

fn load(client: &Client, id: NodeId) -> Task {
    let client = client.clone();
    Task::perform(
        async move {
            let node = client.get(id).await?;
            let versions = match node.kind {
                NodeKind::File => client.versions(id).await?,
                NodeKind::Folder => Vec::new(),
            };
            Ok((node, versions))
        },
        move |result| Message::Loaded(id, result),
    )
}

fn download(client: Client, node: Node, version: i32) -> Task {
    Task::perform(
        async move {
            let Some(handle) = rfd::AsyncFileDialog::new()
                .set_file_name(node.name.as_str())
                .save_file()
                .await
            else {
                return Ok(());
            };
            save(&client, node.id, version, handle.path())
                .await
                .map_err(|err| err.to_string())
        },
        Message::Downloaded,
    )
}

async fn save(
    client: &Client,
    id: NodeId,
    version: i32,
    target: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::io::AsyncWriteExt;

    let mut response = client.download_version(id, version).await?;
    let mut out = tokio::fs::File::create(target).await?;
    while let Some(chunk) = response.chunk().await? {
        out.write_all(&chunk).await?;
    }
    out.flush().await?;
    Ok(())
}
