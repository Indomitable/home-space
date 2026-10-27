use iced::{
    Alignment, Element, Length, Task,
    widget::{button, column, container, operation, row, scrollable, space, text, text_input},
};

use api_client::{
    ApiError, Client,
    format::{conflicts_title, human_size},
    listing::{Sort, sort_nodes},
};
use protocol::{Conflict, MoveResponse, Node, NodeId, NodeKind, OnConflict};

use super::HOME;
use crate::{
    icons::{ICON_SIZE, Icon, Tint},
    locale::short_date,
    theme,
};

const NAME_INPUT: &str = "dialog-name";
const DIALOG_WIDTH: f32 = 420.0;
const CONFLICTS_WIDTH: f32 = 560.0;
const CONFLICTS_HEIGHT: f32 = 280.0;
const META_SIZE: f32 = 13.0;
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
    /// The outcome of a move or restore that may have been held back by clashing files.
    Placed(Result<MoveResponse, ApiError>),
    /// How to resolve the clashes listed by a conflicts dialog.
    Resolve(OnConflict),
}

/// What the browser should do after a dialog update.
pub enum Outcome {
    Task(Task<Message>),
    Closed,
    Done,
    Failed,
}

#[derive(Clone)]
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
    /// Files that clash at a destination; the user decides how to go on.
    Conflicts {
        title: String,
        conflicts: Vec<Conflict>,
        /// What to send again with the chosen mode.
        retry: Retry,
        /// The dialog that held the request, shown again on cancel.
        back: Box<Kind>,
    },
}

/// A request that was held back by clashes and is sent again with a mode.
#[derive(Clone)]
enum Retry {
    Move {
        ids: Vec<NodeId>,
        destination: Option<NodeId>,
    },
}

impl Retry {
    fn send(&self, client: &Client, on_conflict: OnConflict) -> Task<Message> {
        let client = client.clone();
        match self.clone() {
            Retry::Move { ids, destination } => Task::perform(
                async move { client.move_nodes(&ids, destination, on_conflict).await },
                Message::Placed,
            ),
        }
    }
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
                    Kind::Move { .. } | Kind::Delete { .. } | Kind::Conflicts { .. } => {}
                }
                Outcome::Task(Task::none())
            }
            Message::Cancel => match &self.kind {
                Kind::Conflicts { back, .. } => {
                    self.kind = (**back).clone();
                    self.saving = false;
                    self.error = None;
                    Outcome::Task(Task::none())
                }
                _ => Outcome::Closed,
            },
            Message::Resolve(_) if self.saving => Outcome::Task(Task::none()),
            Message::Resolve(mode) => match &self.kind {
                Kind::Conflicts { retry, .. } => {
                    let task = retry.send(client, mode);
                    self.saving = true;
                    self.error = None;
                    Outcome::Task(task)
                }
                _ => Outcome::Task(Task::none()),
            },
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
            Message::Placed(Ok(response)) if response.conflicts.is_empty() => Outcome::Done,
            Message::Placed(Ok(response)) => {
                self.hold_back(response);
                Outcome::Task(Task::none())
            }
            Message::Placed(Err(err)) => self.fail(&err),
            Message::Finished(Ok(())) => Outcome::Done,
            Message::Finished(Err(err)) => self.fail(&err),
        }
    }

    fn fail(&mut self, err: &ApiError) -> Outcome {
        self.saving = false;
        self.error = Some(err.to_string());
        Outcome::Failed
    }

    /// Replaces the move dialog with the list of clashes the server reported.
    fn hold_back(&mut self, response: MoveResponse) {
        let Kind::Move { nodes, trail, .. } = &self.kind else {
            return;
        };
        let retry = Retry::Move {
            ids: ids_of(nodes),
            destination: trail.last().map(|folder| folder.id),
        };
        let name = response
            .target
            .as_ref()
            .map_or(HOME, |folder| folder.name.as_str());
        let title = conflicts_title(response.conflicts.len(), &format!("“{name}”"));
        self.kind = Kind::Conflicts {
            title,
            conflicts: response.conflicts,
            retry,
            back: Box::new(self.kind.clone()),
        };
        self.saving = false;
        self.error = None;
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
                Some(Retry::Move { ids, destination }.send(&client, OnConflict::Abort))
            }
            Kind::Conflicts { .. } => None,
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
            None if matches!(self.kind, Kind::Conflicts { .. }) => Outcome::Task(Task::none()),
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
            Kind::Conflicts {
                title, conflicts, ..
            } => return self.conflicts_view(title, conflicts),
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

    fn conflicts_view<'a>(
        &'a self,
        title: &'a str,
        conflicts: &'a [Conflict],
    ) -> Element<'a, Message> {
        let rows = conflicts.iter().map(|conflict| {
            let existing = conflict
                .existing
                .as_ref()
                .map_or_else(|| "Also selected".to_owned(), describe);
            column![
                text(conflict.source.name.as_str()),
                text(conflict.path.as_str()).size(META_SIZE),
                text(format!("Incoming: {}", describe(&conflict.source))).size(META_SIZE),
                text(format!("Existing: {existing}")).size(META_SIZE),
            ]
            .into()
        });
        let list = scrollable(column(rows).spacing(SPACING).width(Length::Fill))
            .height(CONFLICTS_HEIGHT)
            .width(Length::Fill);
        let choose = |mode| (!self.saving).then_some(Message::Resolve(mode));
        let cancel = button("Cancel")
            .style(button::secondary)
            .on_press(Message::Cancel);
        let skip = button("Skip these files")
            .style(button::secondary)
            .on_press_maybe(choose(OnConflict::Skip));
        let over = button(text(if self.saving {
            "Working…"
        } else {
            "Override"
        }))
        .style(button::danger)
        .on_press_maybe(choose(OnConflict::Override));
        let mut content = column![
            text(title).size(TITLE_SIZE),
            text("Override replaces the current content and keeps it as a version.")
                .size(META_SIZE),
            list,
        ]
        .spacing(SPACING);
        if let Some(error) = &self.error {
            content = content.push(container(text(error.as_str())).style(theme::error));
        }
        content = content.push(row![space::horizontal(), cancel, skip, over].spacing(SPACING));
        container(content)
            .padding(DIALOG_PADDING)
            .width(CONFLICTS_WIDTH)
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

fn describe(node: &Node) -> String {
    format!(
        "{} · {}",
        short_date(node.updated_at),
        human_size(node.size)
    )
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
    sort_nodes(&mut folders, Sort::default());
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

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

    fn node(name: &str, raw: u128) -> Node {
        Node {
            id: NodeId(Uuid::from_u128(raw)),
            parent_id: None,
            kind: NodeKind::File,
            name: name.to_owned(),
            size: 2048,
            sha256: None,
            mime: String::new(),
            version: 1,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    fn clash(response_target: Option<Node>) -> MoveResponse {
        MoveResponse {
            conflicts: vec![Conflict {
                source: node("a.txt", 1),
                path: "dir/a.txt".to_owned(),
                existing: Some(node("a.txt", 2)),
            }],
            target: response_target,
            ..MoveResponse::default()
        }
    }

    fn client() -> Client {
        Client::new("http://localhost")
    }

    fn moving() -> Dialog {
        Dialog::with(Kind::Move {
            nodes: vec![node("a.txt", 1)],
            trail: Vec::new(),
            folders: Some(Vec::new()),
        })
    }

    #[test]
    fn a_move_held_back_by_clashes_shows_them_with_the_target_name() {
        let mut dialog = moving();
        dialog.saving = true;

        let outcome = dialog.update(
            &client(),
            None,
            Message::Placed(Ok(clash(Some(node("Photos", 3))))),
        );

        assert!(matches!(outcome, Outcome::Task(_)));
        assert!(!dialog.saving);
        let Kind::Conflicts {
            title, conflicts, ..
        } = &dialog.kind
        else {
            panic!("expected the conflicts dialog");
        };
        assert_eq!(title, "1 file already exists in “Photos”");
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn clashes_at_the_root_are_titled_with_home() {
        let mut dialog = moving();

        let _ = dialog.update(&client(), None, Message::Placed(Ok(clash(None))));

        let Kind::Conflicts { title, .. } = &dialog.kind else {
            panic!("expected the conflicts dialog");
        };
        assert_eq!(title, "1 file already exists in “Home”");
    }

    #[test]
    fn a_move_without_clashes_is_done() {
        let mut dialog = moving();

        let outcome = dialog.update(
            &client(),
            None,
            Message::Placed(Ok(MoveResponse::default())),
        );

        assert!(matches!(outcome, Outcome::Done));
    }

    #[test]
    fn cancelling_the_conflicts_returns_to_the_move_dialog() {
        let mut dialog = moving();
        let _ = dialog.update(&client(), None, Message::Placed(Ok(clash(None))));

        let outcome = dialog.update(&client(), None, Message::Cancel);

        assert!(matches!(outcome, Outcome::Task(_)));
        assert!(matches!(dialog.kind, Kind::Move { .. }));
        let outcome = dialog.update(&client(), None, Message::Cancel);
        assert!(matches!(outcome, Outcome::Closed));
    }

    #[test]
    fn choosing_a_mode_sends_the_move_again() {
        let mut dialog = moving();
        let _ = dialog.update(&client(), None, Message::Placed(Ok(clash(None))));

        let outcome = dialog.update(&client(), None, Message::Resolve(OnConflict::Skip));

        assert!(matches!(outcome, Outcome::Task(_)));
        assert!(dialog.saving);
        assert!(matches!(dialog.kind, Kind::Conflicts { .. }));
    }

    #[test]
    fn enter_does_not_resolve_clashes() {
        let mut dialog = moving();
        let _ = dialog.update(&client(), None, Message::Placed(Ok(clash(None))));

        let outcome = dialog.update(&client(), None, Message::Submit);

        assert!(matches!(outcome, Outcome::Task(_)));
        assert!(!dialog.saving);
        assert!(matches!(dialog.kind, Kind::Conflicts { .. }));
    }

    #[test]
    fn a_failed_move_keeps_the_dialog_open_with_the_error() {
        let mut dialog = moving();
        dialog.saving = true;

        let outcome = dialog.update(
            &client(),
            None,
            Message::Placed(Err(ApiError::Unauthorized)),
        );

        assert!(matches!(outcome, Outcome::Failed));
        assert!(!dialog.saving);
        assert!(dialog.error.is_some());
    }
}
