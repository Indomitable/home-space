use iced::{
    Alignment, Element, Length, Task,
    widget::{button, column, container, operation, row, scrollable, space, text, text_input},
};

use api_client::{ApiError, Client, listing::sort_nodes};
use protocol::{Node, NodeId, NodeKind};

use crate::{
    icons::{ICON_SIZE, Icon, Tint},
    theme,
};

const NAME_INPUT: &str = "dialog-name";
const DIALOG_WIDTH: f32 = 420.0;
const DIALOG_PADDING: f32 = 20.0;
const PICKER_HEIGHT: f32 = 240.0;
const SPACING: f32 = 12.0;
const TITLE_SIZE: f32 = 20.0;

#[derive(Debug, Clone)]
pub enum Message {
    NameChanged(String),
    Submit,
    Cancel,
    Enter(Node),
    Up,
    Folders(Option<NodeId>, Result<Vec<Node>, ApiError>),
    Finished(Result<(), ApiError>),
}

/// What the browser should do after a dialog update.
pub enum Outcome {
    Task(Task<Message>),
    Closed,
    Done,
    Failed,
}

enum Kind {
    NewFolder {
        name: String,
    },
    Rename {
        node: Node,
        name: String,
    },
    Move {
        nodes: Vec<Node>,
        trail: Vec<Node>,
        folders: Option<Vec<Node>>,
    },
    Delete {
        nodes: Vec<Node>,
    },
}

/// A modal dialog over the browser.
pub struct Dialog {
    kind: Kind,
    saving: bool,
    error: Option<String>,
}

impl Dialog {
    fn with(kind: Kind) -> Self {
        Self {
            kind,
            saving: false,
            error: None,
        }
    }

    /// Asks for the name of a new folder.
    pub fn new_folder() -> (Self, Task<Message>) {
        let dialog = Self::with(Kind::NewFolder {
            name: String::new(),
        });
        (dialog, operation::focus(NAME_INPUT))
    }

    /// Asks for a new name for `node`, with the current name selected.
    pub fn rename(node: Node) -> (Self, Task<Message>) {
        let name = node.name.clone();
        let dialog = Self::with(Kind::Rename { node, name });
        let task = Task::batch([
            operation::focus(NAME_INPUT),
            operation::select_all(NAME_INPUT),
        ]);
        (dialog, task)
    }

    /// Lets the user pick a destination folder for `nodes`.
    pub fn move_nodes(client: &Client, nodes: Vec<Node>) -> (Self, Task<Message>) {
        let task = list_folders(client, None);
        let dialog = Self::with(Kind::Move {
            nodes,
            trail: Vec::new(),
            folders: None,
        });
        (dialog, task)
    }

    /// Confirms deleting `nodes`.
    pub fn delete(nodes: Vec<Node>) -> Self {
        Self::with(Kind::Delete { nodes })
    }

    /// Applies `message`; `folder` is the folder the browser shows.
    pub fn update(&mut self, client: &Client, folder: Option<NodeId>, message: Message) -> Outcome {
        match message {
            Message::NameChanged(value) => {
                match &mut self.kind {
                    Kind::NewFolder { name } | Kind::Rename { name, .. } => *name = value,
                    Kind::Move { .. } | Kind::Delete { .. } => {}
                }
                Outcome::Task(Task::none())
            }
            Message::Cancel => Outcome::Closed,
            Message::Submit if self.saving => Outcome::Task(Task::none()),
            Message::Submit => self.submit(client, folder),
            Message::Enter(target) => self.navigate(client, |trail| trail.push(target)),
            Message::Up => self.navigate(client, |trail| {
                trail.pop();
            }),
            Message::Folders(target, result) => {
                if let Kind::Move {
                    nodes,
                    trail,
                    folders,
                } = &mut self.kind
                    && trail.last().map(|folder| folder.id) == target
                {
                    match result {
                        Ok(listing) => *folders = Some(pickable(listing, nodes)),
                        Err(err) => self.error = Some(err.to_string()),
                    }
                }
                Outcome::Task(Task::none())
            }
            Message::Finished(Ok(())) => Outcome::Done,
            Message::Finished(Err(err)) => {
                self.saving = false;
                self.error = Some(err.to_string());
                Outcome::Failed
            }
        }
    }

    fn navigate(&mut self, client: &Client, change: impl FnOnce(&mut Vec<Node>)) -> Outcome {
        if let Kind::Move { trail, folders, .. } = &mut self.kind {
            change(trail);
            *folders = None;
            self.error = None;
            Outcome::Task(list_folders(client, trail.last().map(|folder| folder.id)))
        } else {
            Outcome::Task(Task::none())
        }
    }

    fn submit(&mut self, client: &Client, folder: Option<NodeId>) -> Outcome {
        let client = client.clone();
        let operation: Option<Task<Message>> = match &self.kind {
            Kind::NewFolder { name } => {
                let name = name.trim().to_owned();
                (!name.is_empty()).then(|| {
                    perform(async move { client.create_folder(folder, &name).await.map(drop) })
                })
            }
            Kind::Rename { node, name } => {
                let name = name.trim().to_owned();
                let id = node.id;
                (!name.is_empty() && name != node.name)
                    .then(|| perform(async move { client.rename(id, &name).await.map(drop) }))
            }
            Kind::Move { nodes, trail, .. } => {
                let ids = ids_of(nodes);
                let destination = trail.last().map(|folder| folder.id);
                Some(perform(async move {
                    for id in ids {
                        client.move_to(id, destination).await?;
                    }
                    Ok(())
                }))
            }
            Kind::Delete { nodes } => {
                let ids = ids_of(nodes);
                Some(perform(async move {
                    for id in ids {
                        client.delete(id).await?;
                    }
                    Ok(())
                }))
            }
        };
        match operation {
            Some(task) => {
                self.saving = true;
                self.error = None;
                Outcome::Task(task)
            }
            None => Outcome::Closed,
        }
    }

    /// The dialog panel, without the backdrop.
    pub fn view(&self) -> Element<'_, Message> {
        let (title, body, action) = match &self.kind {
            Kind::NewFolder { name } => ("New folder".to_owned(), self.name_input(name), "Create"),
            Kind::Rename { node, name } => (
                format!("Rename “{}”", node.name),
                self.name_input(name),
                "Rename",
            ),
            Kind::Move {
                nodes,
                trail,
                folders,
            } => (
                titled("Move", nodes),
                picker(trail, folders.as_deref()),
                "Move here",
            ),
            Kind::Delete { nodes } => (
                titled("Delete", nodes),
                text("This also deletes everything inside folders and can't be undone.").into(),
                "Delete",
            ),
        };
        let confirm_style = match self.kind {
            Kind::Delete { .. } => button::danger,
            _ => button::primary,
        };
        let submit = button(text(if self.saving { "Working…" } else { action }))
            .style(confirm_style)
            .on_press_maybe((!self.saving).then_some(Message::Submit));
        let cancel = button("Cancel")
            .style(button::secondary)
            .on_press(Message::Cancel);
        let mut content = column![text(title).size(TITLE_SIZE), body].spacing(SPACING);
        if let Some(error) = &self.error {
            content = content.push(container(text(error.as_str())).style(theme::error));
        }
        content = content.push(row![space::horizontal(), cancel, submit].spacing(SPACING));
        container(content)
            .padding(DIALOG_PADDING)
            .width(DIALOG_WIDTH)
            .style(theme::panel)
            .into()
    }

    fn name_input<'a>(&self, name: &'a str) -> Element<'a, Message> {
        text_input("Name", name)
            .id(NAME_INPUT)
            .on_input_maybe((!self.saving).then_some(Message::NameChanged))
            .on_submit(Message::Submit)
            .into()
    }
}

fn picker<'a>(trail: &'a [Node], folders: Option<&'a [Node]>) -> Element<'a, Message> {
    let destination = if trail.is_empty() {
        "To Home".to_owned()
    } else {
        let names: Vec<&str> = trail.iter().map(|folder| folder.name.as_str()).collect();
        format!("To /{}", names.join("/"))
    };
    let mut entries = column![].spacing(2);
    if !trail.is_empty() {
        entries = entries.push(picker_entry(Icon::Up, "Up", Message::Up));
    }
    entries = match folders {
        None => entries.push(text("Loading…")),
        Some([]) => entries.push(text("No subfolders")),
        Some(folders) => entries.extend(folders.iter().map(|folder| {
            picker_entry(
                Icon::Folder,
                folder.name.as_str(),
                Message::Enter(folder.clone()),
            )
        })),
    };
    column![
        text(destination),
        scrollable(entries)
            .height(PICKER_HEIGHT)
            .width(Length::Fill),
    ]
    .spacing(SPACING)
    .into()
}

fn picker_entry<'a>(icon: Icon, label: &'a str, message: Message) -> Element<'a, Message> {
    let tint = match icon {
        Icon::Folder => Tint::Accent,
        _ => Tint::Text,
    };
    button(
        row![icon.view(ICON_SIZE, tint), text(label)]
            .spacing(SPACING)
            .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .style(theme::tool)
    .on_press(message)
    .into()
}

fn titled(verb: &str, nodes: &[Node]) -> String {
    match nodes {
        [node] => format!("{verb} “{}”", node.name),
        _ => format!("{verb} {} items", nodes.len()),
    }
}

fn ids_of(nodes: &[Node]) -> Vec<NodeId> {
    nodes.iter().map(|node| node.id).collect()
}

fn pickable(listing: Vec<Node>, moving: &[Node]) -> Vec<Node> {
    let mut folders: Vec<Node> = listing
        .into_iter()
        .filter(|entry| {
            entry.kind == NodeKind::Folder && !moving.iter().any(|node| node.id == entry.id)
        })
        .collect();
    sort_nodes(&mut folders);
    folders
}

fn list_folders(client: &Client, target: Option<NodeId>) -> Task<Message> {
    let client = client.clone();
    Task::perform(async move { client.list(target).await }, move |result| {
        Message::Folders(target, result)
    })
}

fn perform(
    operation: impl Future<Output = Result<(), ApiError>> + Send + 'static,
) -> Task<Message> {
    Task::perform(operation, Message::Finished)
}
