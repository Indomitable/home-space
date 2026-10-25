mod context_menu;
mod dialogs;
mod grid;
mod selection;
mod toolbar;

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use iced::{
    Alignment, Element, Event, Length, Point, Size, Subscription, Task, event,
    keyboard::{self, Key, Modifiers, key::Named},
    mouse,
    widget::{button, center, column, container, mouse_area, opaque, pin, row, space, stack, text},
    window,
};

use api_client::{
    ApiError, Client,
    listing::{Sort, SortKey, affects_path, apply_event, collapsed_crumbs, sort_nodes},
};
use protocol::{Me, Node, NodeId, NodeKind};

use self::{
    dialogs::{Dialog, Outcome},
    selection::Selection,
    toolbar::Action,
};
use crate::{
    events, theme,
    transfers::{self, Batch, Direction, DownloadFile, Transfers, UploadPlan},
};

const PAGE_PADDING: f32 = 16.0;
const SPACING: f32 = 12.0;
const GRID_SPACING: f32 = 4.0;
const USERNAME_SIZE: f32 = 18.0;
const DROP_PADDING: f32 = 32.0;
const DOUBLE_CLICK: Duration = Duration::from_millis(400);
const DEFAULT_WINDOW: Size = Size::new(1024.0, 768.0);
const HOME: &str = "Home";
const CRUMB_SEPARATOR: &str = "›";
const CRUMB_OVERFLOW: &str = "…";
const KEY_SELECT_ALL: &str = "a";
const KEY_NEW_FOLDER: &str = "n";
const KEY_UPLOAD: &str = "u";

/// A keyboard command of the browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shortcut {
    SelectAll,
    Escape,
    Delete,
    Rename,
    Activate,
    Parent,
    NewFolder,
    Upload,
    Refresh,
}

#[derive(Debug, Clone)]
pub enum Message {
    Open(Option<NodeId>),
    Refresh,
    Loaded(Option<NodeId>, Result<Listing, ApiError>),
    Click(NodeId),
    Toggle(NodeId),
    ToggleAll,
    SortBy(SortKey),
    ContextMenu(NodeId),
    CrumbsMenu,
    CloseMenu,
    Act(Action),
    NewFolder,
    Upload,
    UploadPlanned(Option<NodeId>, Result<UploadPlan, String>),
    DownloadPlanned(Result<Vec<DownloadFile>, String>),
    Dialog(dialogs::Message),
    Transfer(transfers::Message),
    Shortcut(Shortcut),
    Modifiers(Modifiers),
    CursorMoved(Point),
    Resized(Size),
    FileHovered,
    FileDropped(PathBuf),
    FilesHoveredLeft,
    Server(protocol::Event),
    SignOut,
    SignedOut,
}

pub enum Update {
    Task(Task<Message>),
    SignedOut,
}

#[derive(Debug, Clone)]
pub struct Listing {
    path: Vec<Node>,
    nodes: Vec<Node>,
}

/// A popup menu and the point it opened at.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Menu {
    Actions(Point),
    Crumbs(Point),
}

pub struct State {
    client: Client,
    me: Me,
    folder: Option<NodeId>,
    listing: Listing,
    sort: Sort,
    loading: bool,
    error: Option<String>,
    selection: Selection,
    modifiers: Modifiers,
    last_click: Option<(NodeId, Instant)>,
    cursor: Point,
    window: Size,
    menu: Option<Menu>,
    dialog: Option<Dialog>,
    transfers: Transfers,
    dropping: bool,
}

impl State {
    pub fn open(client: Client, me: Me) -> (Self, Task<Message>) {
        let state = Self {
            client,
            me,
            folder: None,
            listing: Listing {
                path: Vec::new(),
                nodes: Vec::new(),
            },
            sort: Sort::default(),
            loading: true,
            error: None,
            selection: Selection::default(),
            modifiers: Modifiers::default(),
            last_click: None,
            cursor: Point::ORIGIN,
            window: DEFAULT_WINDOW,
            menu: None,
            dialog: None,
            transfers: Transfers::default(),
            dropping: false,
        };
        let task = state.load();
        (state, task)
    }

    pub fn update(&mut self, message: Message) -> Update {
        match message {
            Message::SignedOut => Update::SignedOut,
            message => Update::Task(self.handle(message)),
        }
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open(folder) => self.open_folder(folder),
            Message::Refresh => {
                self.loading = true;
                self.load()
            }
            Message::Loaded(folder, result) if folder == self.folder => {
                self.loading = false;
                match result {
                    Ok(mut listing) => {
                        sort_nodes(&mut listing.nodes, self.sort);
                        self.selection.retain(&listing.nodes);
                        self.listing = listing;
                        self.error = None;
                    }
                    Err(err) => self.error = Some(err.to_string()),
                }
                Task::none()
            }
            Message::Loaded(..) => Task::none(),
            Message::Click(id) => self.click(id),
            Message::Toggle(id) => {
                self.selection.toggle(id);
                Task::none()
            }
            Message::ToggleAll => {
                self.selection.toggle_all(&self.listing.nodes);
                Task::none()
            }
            Message::SortBy(key) => {
                self.sort = self.sort.toggled(key);
                sort_nodes(&mut self.listing.nodes, self.sort);
                Task::none()
            }
            Message::ContextMenu(id) => {
                self.selection.ensure(id);
                self.menu = Some(Menu::Actions(self.cursor));
                Task::none()
            }
            Message::CrumbsMenu => {
                self.menu = Some(Menu::Crumbs(self.cursor));
                Task::none()
            }
            Message::CloseMenu => {
                self.menu = None;
                Task::none()
            }
            Message::Act(action) => {
                self.menu = None;
                self.act(action)
            }
            Message::NewFolder => {
                let (dialog, task) = Dialog::new_folder();
                self.dialog = Some(dialog);
                task.map(Message::Dialog)
            }
            Message::Upload => self.pick_upload(),
            Message::UploadPlanned(parent, Ok(plan)) if !plan.is_empty() => self
                .transfers
                .enqueue(&self.client, Batch::Upload { parent, plan })
                .map(Message::Transfer),
            Message::DownloadPlanned(Ok(files)) if !files.is_empty() => self
                .transfers
                .enqueue(&self.client, Batch::Download(files))
                .map(Message::Transfer),
            Message::UploadPlanned(_, Err(err)) | Message::DownloadPlanned(Err(err)) => {
                self.error = Some(err);
                Task::none()
            }
            Message::UploadPlanned(..) | Message::DownloadPlanned(_) => Task::none(),
            Message::Dialog(message) => self.dialog_update(message),
            Message::Transfer(message) => {
                let reload = matches!(message, transfers::Message::BatchDone(Direction::Upload));
                let task = self
                    .transfers
                    .update(&self.client, message)
                    .map(Message::Transfer);
                if reload {
                    Task::batch([task, self.load()])
                } else {
                    task
                }
            }
            Message::Shortcut(shortcut) => self.shortcut(shortcut),
            Message::Modifiers(modifiers) => {
                self.modifiers = modifiers;
                Task::none()
            }
            Message::CursorMoved(position) => {
                self.cursor = position;
                Task::none()
            }
            Message::Resized(size) => {
                self.window = size;
                Task::none()
            }
            Message::FileHovered => {
                self.dropping = self.dialog.is_none();
                Task::none()
            }
            Message::FilesHoveredLeft => {
                self.dropping = false;
                Task::none()
            }
            Message::FileDropped(path) => {
                self.dropping = false;
                if self.dialog.is_none() {
                    self.plan_upload(vec![path])
                } else {
                    Task::none()
                }
            }
            Message::Server(event) => {
                if matches!(event, protocol::Event::Resync)
                    || affects_path(&self.listing.path, &event)
                {
                    self.load()
                } else {
                    apply_event(&mut self.listing.nodes, self.folder, &event, self.sort);
                    self.selection.retain(&self.listing.nodes);
                    Task::none()
                }
            }
            Message::SignOut => {
                let client = self.client.clone();
                Task::perform(
                    async move {
                        let _ = client.logout().await;
                    },
                    |()| Message::SignedOut,
                )
            }
            Message::SignedOut => Task::none(),
        }
    }

    fn open_folder(&mut self, folder: Option<NodeId>) -> Task<Message> {
        self.folder = folder;
        self.loading = true;
        self.error = None;
        self.selection.clear();
        self.menu = None;
        self.last_click = None;
        self.load()
    }

    fn click(&mut self, id: NodeId) -> Task<Message> {
        let now = Instant::now();
        let double = self
            .last_click
            .is_some_and(|(last, at)| last == id && now.duration_since(at) <= DOUBLE_CLICK);
        if double {
            self.last_click = None;
            self.activate(id)
        } else {
            self.last_click = Some((id, now));
            self.selection
                .click(id, self.modifiers, &self.listing.nodes);
            Task::none()
        }
    }

    fn activate(&mut self, id: NodeId) -> Task<Message> {
        let node = self
            .listing
            .nodes
            .iter()
            .find(|node| node.id == id)
            .cloned();
        match node {
            Some(node) if node.kind == NodeKind::Folder => self.open_folder(Some(node.id)),
            Some(node) => self.download(vec![node]),
            None => Task::none(),
        }
    }

    fn selected(&self) -> Vec<Node> {
        self.selection
            .selected(&self.listing.nodes)
            .into_iter()
            .cloned()
            .collect()
    }

    fn act(&mut self, action: Action) -> Task<Message> {
        let selected = self.selected();
        match (action, selected.as_slice()) {
            (_, []) => Task::none(),
            (Action::Open, [node]) => self.activate(node.id),
            (Action::Rename, [node]) => {
                let (dialog, task) = Dialog::rename(node.clone());
                self.dialog = Some(dialog);
                task.map(Message::Dialog)
            }
            (Action::Open | Action::Rename, _) => Task::none(),
            (Action::Download, _) => self.download(selected),
            (Action::Move, _) => {
                let (dialog, task) = Dialog::move_nodes(&self.client, selected);
                self.dialog = Some(dialog);
                task.map(Message::Dialog)
            }
            (Action::Delete, _) => {
                self.dialog = Some(Dialog::delete(selected));
                Task::none()
            }
        }
    }

    fn dialog_update(&mut self, message: dialogs::Message) -> Task<Message> {
        let Some(dialog) = &mut self.dialog else {
            return Task::none();
        };
        match dialog.update(&self.client, self.folder, message) {
            Outcome::Task(task) => task.map(Message::Dialog),
            Outcome::Closed => {
                self.dialog = None;
                Task::none()
            }
            Outcome::Done => {
                self.dialog = None;
                self.selection.clear();
                self.load()
            }
            Outcome::Failed => self.load(),
        }
    }

    fn shortcut(&mut self, shortcut: Shortcut) -> Task<Message> {
        if self.dialog.is_some() {
            match shortcut {
                Shortcut::Escape => self.dialog_update(dialogs::Message::Cancel),
                Shortcut::Activate => self.dialog_update(dialogs::Message::Submit),
                _ => Task::none(),
            }
        } else if self.menu.take().is_some() && shortcut == Shortcut::Escape {
            Task::none()
        } else {
            match shortcut {
                Shortcut::Escape => {
                    self.selection.clear();
                    Task::none()
                }
                Shortcut::SelectAll => {
                    self.selection.select_all(&self.listing.nodes);
                    Task::none()
                }
                Shortcut::Delete => self.act(Action::Delete),
                Shortcut::Rename => self.act(Action::Rename),
                Shortcut::Activate => match self.selected().as_slice() {
                    [node] => self.activate(node.id),
                    _ => Task::none(),
                },
                Shortcut::Parent => match self.parent() {
                    Some(parent) => self.open_folder(parent),
                    None => Task::none(),
                },
                Shortcut::NewFolder => self.handle(Message::NewFolder),
                Shortcut::Upload => self.pick_upload(),
                Shortcut::Refresh => self.handle(Message::Refresh),
            }
        }
    }

    fn parent(&self) -> Option<Option<NodeId>> {
        match self.listing.path.as_slice() {
            [] => None,
            [.., parent, _] => Some(Some(parent.id)),
            [_] => Some(None),
        }
    }

    fn pick_upload(&self) -> Task<Message> {
        let parent = self.folder;
        Task::perform(
            async move {
                match rfd::AsyncFileDialog::new().pick_files().await {
                    Some(handles) => {
                        let paths = handles
                            .iter()
                            .map(|handle| handle.path().to_path_buf())
                            .collect();
                        transfers::plan_upload(paths).await
                    }
                    None => Ok(UploadPlan::default()),
                }
            },
            move |result| Message::UploadPlanned(parent, result),
        )
    }

    fn plan_upload(&self, paths: Vec<PathBuf>) -> Task<Message> {
        let parent = self.folder;
        Task::perform(transfers::plan_upload(paths), move |result| {
            Message::UploadPlanned(parent, result)
        })
    }

    fn download(&self, nodes: Vec<Node>) -> Task<Message> {
        match nodes.as_slice() {
            [node] if node.kind == NodeKind::File => {
                let node = node.clone();
                Task::perform(
                    async move {
                        let handle = rfd::AsyncFileDialog::new()
                            .set_file_name(&node.name)
                            .save_file()
                            .await;
                        Ok(handle
                            .map(|handle| {
                                vec![DownloadFile::replacing(&node, handle.path().to_path_buf())]
                            })
                            .unwrap_or_default())
                    },
                    Message::DownloadPlanned,
                )
            }
            _ => {
                let client = self.client.clone();
                Task::perform(
                    async move {
                        match rfd::AsyncFileDialog::new().pick_folder().await {
                            Some(folder) => {
                                transfers::plan_download(client, nodes, folder.path().to_path_buf())
                                    .await
                            }
                            None => Ok(Vec::new()),
                        }
                    },
                    Message::DownloadPlanned,
                )
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let selected = self.selection.selected(&self.listing.nodes);
        let header = row![
            text(self.me.username.as_str()).size(USERNAME_SIZE),
            space::horizontal(),
            button("Sign out")
                .style(button::secondary)
                .on_press(Message::SignOut),
        ]
        .spacing(SPACING)
        .align_y(Alignment::Center);

        let mut page =
            column![header, self.breadcrumbs(), toolbar::view(&selected)].spacing(SPACING);
        if let Some(error) = &self.error {
            page = page.push(container(text(error.as_str())).style(theme::error));
        }
        let grid = column![
            grid::header(self.selection.coverage(&self.listing.nodes), self.sort),
            grid::rows(&self.listing.nodes, &self.selection),
        ]
        .spacing(GRID_SPACING);
        let page = container(page.push(grid))
            .padding(PAGE_PADDING)
            .width(Length::Fill)
            .height(Length::Fill);

        let mut layers: Vec<Element<'_, Message>> = vec![page.into()];
        if !self.transfers.is_empty() {
            layers.push(
                container(self.transfers.view().map(Message::Transfer))
                    .padding(PAGE_PADDING)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_right(Length::Fill)
                    .align_bottom(Length::Fill)
                    .into(),
            );
        }
        if let Some(menu) = self.menu {
            let (cursor, items, panel) = match menu {
                Menu::Actions(cursor) => (
                    cursor,
                    context_menu::item_count(&selected),
                    context_menu::view(&selected),
                ),
                Menu::Crumbs(cursor) => {
                    let hidden = self.hidden_crumbs();
                    (cursor, hidden.len(), context_menu::folders(hidden))
                }
            };
            let position = context_menu::position(cursor, self.window, items);
            layers.push(
                mouse_area(space().width(Length::Fill).height(Length::Fill))
                    .on_press(Message::CloseMenu)
                    .on_right_press(Message::CloseMenu)
                    .into(),
            );
            layers.push(pin(panel).position(position).into());
        }
        if let Some(dialog) = &self.dialog {
            layers.push(modal(
                dialog.view().map(Message::Dialog),
                Message::Dialog(dialogs::Message::Cancel),
            ));
        }
        if self.dropping {
            let target = self
                .listing
                .path
                .last()
                .map_or(HOME, |folder| folder.name.as_str());
            let label = container(text(format!("Drop files to upload to {target}")))
                .padding(DROP_PADDING)
                .style(theme::panel);
            layers.push(center(label).style(theme::backdrop).into());
        }
        stack(layers).into()
    }

    fn hidden_crumbs(&self) -> &[Node] {
        collapsed_crumbs(self.listing.path.len())
            .and_then(|hidden| self.listing.path.get(hidden))
            .unwrap_or_default()
    }

    fn breadcrumbs(&self) -> Element<'_, Message> {
        let hidden = collapsed_crumbs(self.listing.path.len()).unwrap_or_default();
        let crumbs = self
            .listing
            .path
            .iter()
            .enumerate()
            .filter_map(|(index, folder)| {
                if index == hidden.start && !hidden.is_empty() {
                    Some(crumb(CRUMB_OVERFLOW, Message::CrumbsMenu))
                } else if hidden.contains(&index) {
                    None
                } else {
                    Some(crumb(folder.name.as_str(), Message::Open(Some(folder.id))))
                }
            })
            .flatten();
        let mut trail = row![
            button(text(HOME))
                .style(theme::tool)
                .on_press(Message::Open(None))
        ]
        .extend(crumbs)
        .spacing(GRID_SPACING)
        .align_y(Alignment::Center);
        if self.loading {
            trail = trail.push(space::horizontal()).push(text("Loading…"));
        }
        trail.into()
    }

    /// Input events and live server events while the browser is shown.
    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            event::listen_with(input),
            events::subscription(&self.client).map(Message::Server),
        ])
    }

    fn load(&self) -> Task<Message> {
        let folder = self.folder;
        Task::perform(fetch(self.client.clone(), folder), move |result| {
            Message::Loaded(folder, result)
        })
    }
}

fn modal<'a>(content: Element<'a, Message>, on_dismiss: Message) -> Element<'a, Message> {
    opaque(mouse_area(center(opaque(content)).style(theme::backdrop)).on_press(on_dismiss))
}

/// A separator followed by a link-styled crumb.
fn crumb(label: &str, message: Message) -> [Element<'_, Message>; 2] {
    [
        text(CRUMB_SEPARATOR).into(),
        button(text(label))
            .style(theme::tool)
            .on_press(message)
            .into(),
    ]
}

fn input(event: Event, status: event::Status, _window: window::Id) -> Option<Message> {
    match event {
        Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })
            if status == event::Status::Ignored =>
        {
            shortcut(&key, modifiers).map(Message::Shortcut)
        }
        Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
            Some(Message::Modifiers(modifiers))
        }
        Event::Mouse(mouse::Event::CursorMoved { position }) => {
            Some(Message::CursorMoved(position))
        }
        Event::Window(window::Event::Resized(size)) => Some(Message::Resized(size)),
        Event::Window(window::Event::FileHovered(_)) => Some(Message::FileHovered),
        Event::Window(window::Event::FileDropped(path)) => Some(Message::FileDropped(path)),
        Event::Window(window::Event::FilesHoveredLeft) => Some(Message::FilesHoveredLeft),
        _ => None,
    }
}

fn shortcut(key: &Key, modifiers: Modifiers) -> Option<Shortcut> {
    match key.as_ref() {
        Key::Named(Named::Escape) => Some(Shortcut::Escape),
        Key::Named(Named::Delete) => Some(Shortcut::Delete),
        Key::Named(Named::F2) => Some(Shortcut::Rename),
        Key::Named(Named::F5) => Some(Shortcut::Refresh),
        Key::Named(Named::Enter) => Some(Shortcut::Activate),
        Key::Named(Named::Backspace) => Some(Shortcut::Parent),
        Key::Named(Named::ArrowUp) if modifiers.alt() => Some(Shortcut::Parent),
        Key::Character(KEY_SELECT_ALL) if modifiers.command() => Some(Shortcut::SelectAll),
        Key::Character(KEY_NEW_FOLDER) if modifiers.command() => Some(Shortcut::NewFolder),
        Key::Character(KEY_UPLOAD) if modifiers.command() => Some(Shortcut::Upload),
        _ => None,
    }
}

async fn fetch(client: Client, folder: Option<NodeId>) -> Result<Listing, ApiError> {
    let (nodes, path) = match folder {
        None => (client.list(None).await?, Vec::new()),
        Some(id) => {
            let (nodes, current, mut path) =
                tokio::try_join!(client.list(Some(id)), client.get(id), client.ancestors(id))?;
            path.push(current);
            (nodes, path)
        }
    };
    Ok(Listing { path, nodes })
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use iced_test::simulator;

    use super::*;

    const FOLDER_NAME: &str = "Folder B";
    const FILE_NAME: &str = "report.pdf";
    const OTHER_FILE_NAME: &str = "notes.txt";
    const FOLDER_ID: u128 = 1;
    const FILE_ID: u128 = 2;
    const OTHER_FILE_ID: u128 = 3;

    fn id(raw: u128) -> NodeId {
        NodeId(Uuid::from_u128(raw))
    }

    fn node(name: &str, kind: NodeKind, raw: u128) -> Node {
        Node {
            id: id(raw),
            parent_id: None,
            kind,
            name: name.to_owned(),
            size: 2048,
            sha256: None,
            mime: None,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    fn state_with_listing() -> State {
        let me = Me {
            id: protocol::UserId(Uuid::nil()),
            username: "alice".to_owned(),
        };
        let (mut state, _task) = State::open(Client::new("http://localhost"), me);
        let listing = Listing {
            path: Vec::new(),
            nodes: vec![
                node(FOLDER_NAME, NodeKind::Folder, FOLDER_ID),
                node(FILE_NAME, NodeKind::File, FILE_ID),
                node(OTHER_FILE_NAME, NodeKind::File, OTHER_FILE_ID),
            ],
        };
        let _ = state.update(Message::Loaded(None, Ok(listing)));
        state
    }

    #[test]
    fn clicking_a_row_selects_it() {
        let state = state_with_listing();
        let mut ui = simulator(state.view());

        ui.click(FILE_NAME).expect("file row is shown");

        let messages: Vec<_> = ui.into_messages().collect();
        assert!(
            matches!(messages.as_slice(), [Message::Click(clicked)] if *clicked == id(FILE_ID))
        );
    }

    #[test]
    fn a_click_selects_only_that_row() {
        let mut state = state_with_listing();

        let _ = state.update(Message::Click(id(FILE_ID)));

        assert_eq!(state.selected().len(), 1);
        assert!(state.selection.contains(id(FILE_ID)));
    }

    #[test]
    fn double_clicking_a_folder_opens_it() {
        let mut state = state_with_listing();

        let _ = state.update(Message::Click(id(FOLDER_ID)));
        let _ = state.update(Message::Click(id(FOLDER_ID)));

        assert_eq!(state.folder, Some(id(FOLDER_ID)));
        assert!(state.loading);
    }

    #[test]
    fn rename_is_disabled_for_two_selected_items() {
        let mut state = state_with_listing();
        let _ = state.update(Message::Toggle(id(FILE_ID)));
        let _ = state.update(Message::Toggle(id(OTHER_FILE_ID)));
        let mut ui = simulator(state.view());

        ui.click("Rename").expect("rename is shown");

        assert_eq!(ui.into_messages().count(), 0);
    }

    #[test]
    fn context_menu_delete_opens_the_delete_dialog() {
        let mut state = state_with_listing();

        let _ = state.update(Message::ContextMenu(id(FILE_ID)));
        let _ = state.update(Message::Act(Action::Delete));

        assert!(state.menu.is_none());
        assert!(state.dialog.is_some());
    }

    #[test]
    fn escape_closes_the_dialog() {
        let mut state = state_with_listing();
        let _ = state.update(Message::NewFolder);

        let _ = state.update(Message::Shortcut(Shortcut::Escape));

        assert!(state.dialog.is_none());
    }

    #[test]
    fn creating_a_folder_with_an_empty_name_closes_the_dialog() {
        let mut state = state_with_listing();
        let _ = state.update(Message::NewFolder);

        let _ = state.update(Message::Dialog(dialogs::Message::Submit));

        assert!(state.dialog.is_none());
    }

    #[test]
    fn backspace_goes_to_the_parent_folder() {
        let mut state = state_with_listing();
        let _ = state.update(Message::Open(Some(id(FOLDER_ID))));
        let listing = Listing {
            path: vec![node(FOLDER_NAME, NodeKind::Folder, FOLDER_ID)],
            nodes: Vec::new(),
        };
        let _ = state.update(Message::Loaded(Some(id(FOLDER_ID)), Ok(listing)));

        let _ = state.update(Message::Shortcut(Shortcut::Parent));

        assert_eq!(state.folder, None);
    }

    #[test]
    fn shortcuts_need_the_command_modifier_for_letters() {
        let key = Key::Character(KEY_SELECT_ALL.into());
        assert_eq!(shortcut(&key, Modifiers::empty()), None);
        assert_eq!(
            shortcut(&key, Modifiers::COMMAND),
            Some(Shortcut::SelectAll)
        );
    }

    fn open_deep_path(state: &mut State, depth: u128) {
        let path: Vec<Node> = (1..=depth)
            .map(|raw| node(&format!("level-{raw}"), NodeKind::Folder, 100 + raw))
            .collect();
        let current = path.last().map(|folder| folder.id);
        let _ = state.update(Message::Open(current));
        let listing = Listing {
            path,
            nodes: Vec::new(),
        };
        let _ = state.update(Message::Loaded(current, Ok(listing)));
    }

    #[test]
    fn deep_breadcrumbs_hide_the_middle_folders() {
        let mut state = state_with_listing();
        open_deep_path(&mut state, 5);
        let mut ui = simulator(state.view());

        assert!(ui.find("level-1").is_ok());
        assert!(ui.find("level-5").is_ok());
        assert!(ui.find("level-3").is_err());
        assert_eq!(state.hidden_crumbs().len(), 3);
    }

    #[test]
    fn the_overflow_crumb_opens_the_hidden_folders_menu() {
        let mut state = state_with_listing();
        open_deep_path(&mut state, 4);

        let _ = state.update(Message::CrumbsMenu);
        let mut ui = simulator(state.view());

        assert!(matches!(state.menu, Some(Menu::Crumbs(_))));
        assert!(ui.find("level-2").is_ok());
    }

    #[test]
    fn a_created_node_in_the_open_folder_appears() {
        let mut state = state_with_listing();
        let created = node("added.txt", NodeKind::File, 9);

        let update = state.update(Message::Server(protocol::Event::NodeCreated {
            node: created,
        }));

        assert!(matches!(update, Update::Task(task) if task.units() == 0));
        assert!(state.listing.nodes.iter().any(|node| node.id == id(9)));
    }

    #[test]
    fn a_deleted_selected_node_leaves_the_selection() {
        let mut state = state_with_listing();
        let _ = state.update(Message::Click(id(FILE_ID)));

        let _ = state.update(Message::Server(protocol::Event::NodeDeleted {
            id: id(FILE_ID),
            parent_id: None,
        }));

        assert!(state.selected().is_empty());
    }

    #[test]
    fn a_resync_reloads_the_listing() {
        let mut state = state_with_listing();

        let update = state.update(Message::Server(protocol::Event::Resync));

        assert!(matches!(update, Update::Task(task) if task.units() > 0));
    }
}
