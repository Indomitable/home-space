use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::Arc,
};

use iced::{
    Alignment, Element, Length, Task,
    widget::{button, column, container, row, scrollable, space, text},
};

use client_core::labels::HOME;
use protocol::{Node, NodeId, NodeKind};

use super::{
    Message as BrowserMessage,
    context_menu::Entry,
    sync_queue::{problem_row, retry_all},
};
use crate::{
    icons::{ICON_SIZE, Icon, Tint},
    locale::short_date,
    sync::{self, Activity, Badge, Command, Commands, Mapping, Outage, Status},
    theme,
};

const PANEL_WIDTH: f32 = 560.0;
const PANEL_PADDING: f32 = 16.0;
const SPACING: f32 = 10.0;
const TITLE_SIZE: f32 = 18.0;
const NOTE_SIZE: f32 = 13.0;
const PROBLEMS_HEIGHT: f32 = 160.0;
const SHOWN_PROBLEMS: usize = 50;
const SHOW_ALL: &str = "Show all";
const RETRY_NOW: &str = "Retry now";
const VERIFY: &str = "Verify";
const VERIFYING: &str = "Verifying…";

#[derive(Debug, Clone)]
pub enum Message {
    /// A report of the sync engine.
    Engine(sync::Update),
    /// Asks for the local folder a remote folder (`None` for Home) syncs with.
    Pick(Option<NodeId>),
    Picked(Option<NodeId>, Option<PathBuf>),
    Measured(Confirm),
    /// Whether to go ahead with the picked folder.
    Confirm(bool),
    Set(Mapping),
    Unmap(Option<NodeId>),
    OpenLocal(PathBuf),
    Settings(bool),
    /// Tries the failed paths again now: these ones, or all of them with `None`.
    Retry(Option<Vec<PathBuf>>),
    /// Reads a synced folder (`None` for Home) from scratch on both sides.
    Verify(Option<NodeId>),
}

/// What the browser has to do after a sync update.
pub enum Outcome {
    Task(Task<Message>),
    /// Something to tell the user.
    Error(String),
}

/// What the browser knows of the sync engine.
#[derive(Debug, Default)]
pub struct Syncing {
    commands: Option<Commands>,
    status: Arc<Status>,
    /// Whether the sync settings are shown.
    pub settings: bool,
    /// A picked folder waiting for the user to confirm it.
    pub confirm: Option<Confirm>,
    /// Why the last mapping was refused.
    pub notice: Option<String>,
}

/// A local folder picked for a remote one, shown before syncing starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirm {
    pub remote: Option<NodeId>,
    /// The remote folder's name.
    pub name: String,
    pub local: PathBuf,
    pub contents: Contents,
}

/// What a local folder already holds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Contents {
    pub files: u64,
    pub bytes: u64,
}

impl Syncing {
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Applies `message`; `listed` names a folder (not Home) the listing knows.
    pub fn update(
        &mut self,
        message: Message,
        listed: impl FnOnce(NodeId) -> Option<String>,
    ) -> Outcome {
        match message {
            Message::Engine(update) => match update {
                sync::Update::Ready(commands) => self.commands = Some(commands),
                sync::Update::Status(status) => self.status = status,
                sync::Update::Rejected(reason) => {
                    self.notice = Some(reason.clone());
                    return Outcome::Error(format!("Could not sync there: {reason}"));
                }
            },
            Message::Pick(remote) => {
                return Outcome::Task(Task::perform(pick_folder(), move |local| {
                    Message::Picked(remote, local)
                }));
            }
            Message::Picked(remote, Some(local)) => {
                let name = match remote {
                    None => HOME.to_owned(),
                    Some(id) => listed(id)
                        .or_else(|| self.status.names.get(&id).cloned())
                        .unwrap_or_else(|| "this folder".to_owned()),
                };
                return Outcome::Task(Task::perform(measure(local.clone()), move |contents| {
                    Message::Measured(Confirm {
                        remote,
                        name,
                        local,
                        contents,
                    })
                }));
            }
            Message::Picked(_, None) => {}
            Message::Measured(confirm) => self.confirm = Some(confirm),
            Message::Confirm(true) => self.confirmed(),
            Message::Confirm(false) => self.confirm = None,
            Message::Set(mapping) => self.send(Command::Map(mapping)),
            Message::Unmap(remote) => self.send(Command::Unmap(remote)),
            Message::OpenLocal(path) => {
                if let Err(err) = reveal(&path) {
                    return Outcome::Error(err);
                }
            }
            Message::Settings(open) => self.settings = open,
            Message::Retry(paths) => self.send(Command::Retry(paths)),
            Message::Verify(remote) => self.send(Command::Verify(remote)),
        }
        Outcome::Task(Task::none())
    }

    /// Starts syncing what was confirmed.
    fn confirmed(&mut self) {
        if let Some(confirm) = self.confirm.take() {
            self.notice = None;
            self.send(Command::Map(Mapping {
                remote: confirm.remote,
                local: Some(confirm.local),
            }));
        }
    }

    pub fn send(&self, command: Command) {
        if let Some(commands) = &self.commands {
            commands.send(command);
        }
    }

    /// The sync entries of the context menu for `selected`: only a single folder has any.
    pub fn entries(&self, selected: &[&Node]) -> Vec<Entry> {
        let [node] = selected else {
            return Vec::new();
        };
        if node.kind != NodeKind::Folder {
            return Vec::new();
        }
        let id = Some(node.id);
        match self.status.mapping(id).map(|mapping| mapping.local.clone()) {
            Some(Some(local)) => vec![
                Entry {
                    icon: Icon::FolderOpen,
                    label: "Open local folder",
                    message: BrowserMessage::Sync(Message::OpenLocal(local)),
                },
                Entry {
                    icon: Icon::Sync,
                    label: "Change local folder…",
                    message: BrowserMessage::Sync(Message::Pick(id)),
                },
                Entry {
                    icon: Icon::SyncDisabled,
                    label: "Stop syncing",
                    message: BrowserMessage::Sync(Message::Unmap(id)),
                },
            ],
            Some(None) => vec![Entry {
                icon: Icon::Sync,
                label: "Sync again",
                message: BrowserMessage::Sync(Message::Unmap(id)),
            }],
            None => {
                let mut entries = vec![Entry {
                    icon: Icon::Sync,
                    label: "Sync to local folder…",
                    message: BrowserMessage::Sync(Message::Pick(id)),
                }];
                // Only a folder synced through a parent can be left out of it.
                if self.status.badge(id).is_some() {
                    entries.push(Entry {
                        icon: Icon::SyncDisabled,
                        label: "Don't sync",
                        message: BrowserMessage::Sync(Message::Set(Mapping {
                            remote: id,
                            local: None,
                        })),
                    });
                }
                entries
            }
        }
    }
}

/// The icon and tint of `badge`.
pub fn badge_icon(badge: Badge) -> (Icon, Tint) {
    match badge {
        Badge::Synced => (Icon::Synced, Tint::Success),
        Badge::Pending => (Icon::SyncPending, Tint::Accent),
        Badge::Problem => (Icon::SyncProblem, Tint::Danger),
        Badge::Disabled => (Icon::SyncDisabled, Tint::Muted),
    }
}

/// The header button summing up the sync, which opens its settings.
pub fn indicator(status: &Status) -> Element<'_, BrowserMessage> {
    let busy = match status.activity {
        Activity::Checking => Some("Checking server…"),
        Activity::Scanning => Some("Scanning…"),
        Activity::Idle | Activity::Syncing => None,
    };
    let (icon, tint, label) = match status.overall() {
        _ if status.pending == 0 && busy.is_some() => (
            Icon::SyncPending,
            Tint::Accent,
            busy.unwrap_or_default().to_owned(),
        ),
        None => (Icon::Sync, Tint::Muted, "Sync".to_owned()),
        Some(Badge::Synced) => (Icon::Synced, Tint::Success, "Up to date".to_owned()),
        Some(Badge::Pending) => (
            Icon::SyncPending,
            Tint::Accent,
            format!("Syncing {}", status.pending),
        ),
        Some(Badge::Problem | Badge::Disabled) if status.outage.is_some() => (
            Icon::SyncProblem,
            Tint::Danger,
            "Server unreachable".to_owned(),
        ),
        Some(Badge::Problem | Badge::Disabled) => {
            (Icon::SyncProblem, Tint::Danger, "Sync problem".to_owned())
        }
    };
    button(
        row![icon.view(ICON_SIZE, tint), text(label)]
            .spacing(SPACING / 2.0)
            .align_y(Alignment::Center),
    )
    .style(theme::tool)
    .on_press(BrowserMessage::Sync(Message::Settings(true)))
    .into()
}

/// Asks before syncing a remote folder with the picked local folder, showing what it holds.
pub fn confirm(confirm: &Confirm) -> Element<'_, BrowserMessage> {
    let name = confirm.name.as_str();
    let buttons = row![
        space::horizontal(),
        button(text("Cancel"))
            .style(button::secondary)
            .on_press(BrowserMessage::Sync(Message::Confirm(false))),
        button(text("Sync")).on_press(BrowserMessage::Sync(Message::Confirm(true))),
    ]
    .spacing(SPACING);
    let panel = column![
        text(format!("Sync “{name}” with this folder?")).size(TITLE_SIZE),
        text(display_path(&confirm.local)),
        buttons,
    ]
    .spacing(SPACING);
    container(panel)
        .padding(PANEL_PADDING)
        .width(PANEL_WIDTH)
        .style(theme::panel)
        .into()
}

/// Counts the files under `path` and their size.
pub async fn measure(path: PathBuf) -> Contents {
    tokio::task::spawn_blocking(move || {
        let mut contents = Contents::default();
        let mut pending = vec![path];
        while let Some(dir) = pending.pop() {
            let Ok(listing) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in listing.flatten() {
                let Ok(meta) = entry.metadata() else {
                    continue;
                };
                if meta.is_dir() {
                    pending.push(entry.path());
                } else if meta.is_file() {
                    contents.files += 1;
                    contents.bytes += meta.len();
                }
            }
        }
        contents
    })
    .await
    .unwrap_or_default()
}

/// The sync settings: what syncs where, and what went wrong.
pub fn settings<'a>(status: &'a Status, notice: Option<&'a str>) -> Element<'a, BrowserMessage> {
    let close = button(Icon::Close.view(ICON_SIZE, Tint::Muted))
        .style(theme::tool)
        .on_press(BrowserMessage::Sync(Message::Settings(false)));
    let mut panel = column![
        row![text("Sync").size(TITLE_SIZE), space::horizontal(), close].align_y(Alignment::Center)
    ]
    .spacing(SPACING);
    for error in status.error.as_deref().into_iter().chain(notice) {
        panel = panel.push(container(text(error)).style(theme::error));
    }
    if let Some(outage) = &status.outage {
        let retry = tool(
            Icon::Refresh,
            RETRY_NOW,
            BrowserMessage::Sync(Message::Retry(None)),
        );
        panel = panel.push(
            container(
                row![text(outage_text(outage)).width(Length::Fill), retry]
                    .spacing(SPACING)
                    .align_y(Alignment::Center),
            )
            .style(theme::error),
        );
    }
    if status.mapping(None).is_none() {
        panel = panel.push(mapping_row(HOME.to_owned(), None, None, false));
    }
    let mut mappings: Vec<&Mapping> = status.mappings.iter().collect();
    mappings.sort_by_key(|mapping| mapping.remote.map(|id| folder_name(status, id)));
    for mapping in mappings {
        let name = mapping
            .remote
            .map_or_else(|| HOME.to_owned(), |id| folder_name(status, id));
        let verifying = status.is_verifying(mapping.remote);
        panel = panel.push(mapping_row(name, Some(mapping), mapping.remote, verifying));
    }
    panel = panel.push(
        text("Right-click a folder to sync it to a location of its own, or to leave it out.")
            .size(NOTE_SIZE),
    );
    if !status.problems.is_empty() {
        let problems = status
            .problems
            .iter()
            .take(SHOWN_PROBLEMS)
            .map(|(path, fault)| problem_row(path, fault));
        let heading = row![
            text(format!(
                "{} could not be synced",
                plural(status.problems.len())
            ))
            .width(Length::Fill),
            retry_all(),
            tool(Icon::Info, SHOW_ALL, BrowserMessage::OpenSyncQueue),
        ]
        .spacing(SPACING / 2.0)
        .align_y(Alignment::Center);
        panel = panel
            .push(heading)
            .push(scrollable(column(problems)).height(PROBLEMS_HEIGHT));
    }
    container(panel)
        .padding(PANEL_PADDING)
        .width(PANEL_WIDTH)
        .style(theme::panel)
        .into()
}

/// A mapped folder (or Home, unmapped) with what can be done with it.
fn mapping_row(
    name: String,
    mapping: Option<&Mapping>,
    remote: Option<NodeId>,
    verifying: bool,
) -> Element<'_, BrowserMessage> {
    let target = match mapping.map(|mapping| mapping.local.as_deref()) {
        None => "Not synced".to_owned(),
        Some(None) => "Set to not sync".to_owned(),
        Some(Some(local)) => display_path(local),
    };
    let mut actions = row![].spacing(SPACING / 2.0);
    match mapping {
        None => {
            actions = actions.push(tool(
                Icon::Sync,
                "Choose folder…",
                BrowserMessage::Sync(Message::Pick(remote)),
            ));
        }
        Some(mapping) => {
            if let Some(local) = &mapping.local {
                actions = actions.push(tool(
                    Icon::FolderOpen,
                    "Open",
                    BrowserMessage::Sync(Message::OpenLocal(local.clone())),
                ));
                actions = actions.push(verify_tool(remote, verifying));
            }
            actions = actions.push(tool(
                Icon::Close,
                "Remove",
                BrowserMessage::Sync(Message::Unmap(remote)),
            ));
        }
    }
    row![
        column![text(name), text(target).size(NOTE_SIZE)].width(Length::Fill),
        actions
    ]
    .spacing(SPACING)
    .align_y(Alignment::Center)
    .into()
}

fn tool(icon: Icon, label: &str, message: BrowserMessage) -> Element<'_, BrowserMessage> {
    button(
        row![icon.view(ICON_SIZE, Tint::Text), text(label)]
            .spacing(SPACING / 2.0)
            .align_y(Alignment::Center),
    )
    .style(theme::tool)
    .on_press(message)
    .into()
}

/// Verifies the synced folder `remote`; inactive while it is being verified.
fn verify_tool<'a>(remote: Option<NodeId>, verifying: bool) -> Element<'a, BrowserMessage> {
    let label = if verifying { VERIFYING } else { VERIFY };
    button(
        row![Icon::Synced.view(ICON_SIZE, Tint::Text), text(label)]
            .spacing(SPACING / 2.0)
            .align_y(Alignment::Center),
    )
    .style(theme::tool)
    .on_press_maybe((!verifying).then_some(BrowserMessage::Sync(Message::Verify(remote))))
    .into()
}

/// "Cannot reach the server since …: why. Trying again at …".
fn outage_text(outage: &Outage) -> String {
    format!(
        "Cannot reach the server since {}: {}. Trying again at {}.",
        short_date(outage.since),
        outage.reason,
        short_date(outage.retry_at)
    )
}

fn folder_name(status: &Status, id: NodeId) -> String {
    status
        .names
        .get(&id)
        .cloned()
        .unwrap_or_else(|| "A removed folder".to_owned())
}

fn plural(count: usize) -> String {
    if count == 1 {
        "1 item".to_owned()
    } else {
        format!("{count} items")
    }
}

/// `path` for display, with the home folder shortened to `~`.
pub fn display_path(path: &Path) -> String {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    match home
        .as_deref()
        .and_then(|home| path.strip_prefix(home).ok())
    {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// Opens `path` in the system's file manager.
pub fn reveal(path: &Path) -> Result<(), String> {
    open_external(path.as_os_str(), &path.display().to_string())
}

/// Opens `url` in the default web browser.
pub fn open_url(url: &str) -> Result<(), String> {
    open_external(OsStr::new(url), url)
}

/// Hands `target` to the system to open; `name` is what an error calls it.
fn open_external(target: &OsStr, name: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    const OPENER: &str = "open";
    #[cfg(target_os = "windows")]
    const OPENER: &str = "explorer";
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    const OPENER: &str = "xdg-open";
    std::process::Command::new(OPENER)
        .arg(target)
        .spawn()
        .map(drop)
        .map_err(|err| format!("could not open {name}: {err}"))
}

/// Asks for the local folder to sync with.
pub async fn pick_folder() -> Option<PathBuf> {
    // A single pick reads the panel's URL, which on macOS can be the parent of a folder that
    // was just created in the panel; the list of selected folders is reliable.
    rfd::AsyncFileDialog::new()
        .set_title("Choose the local folder to sync with")
        .pick_folders()
        .await?
        .into_iter()
        .next()
        .map(|folder| folder.path().to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

    fn folder(n: u128) -> Node {
        Node {
            id: NodeId(Uuid::from_u128(n)),
            parent_id: None,
            kind: NodeKind::Folder,
            name: format!("f{n}"),
            size: 0,
            sha256: None,
            mime: "inode/directory".to_owned(),
            version: 1,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
            edited_on: None,
        }
    }

    fn syncing(mappings: Vec<Mapping>, badges: Vec<(Node, Badge)>) -> Syncing {
        Syncing {
            status: Arc::new(Status {
                mappings,
                badges: badges
                    .into_iter()
                    .map(|(node, badge)| (Some(node.id), badge))
                    .collect(),
                names: HashMap::new(),
                ..Status::default()
            }),
            ..Syncing::default()
        }
    }

    fn labels(syncing: &Syncing, node: &Node) -> Vec<&'static str> {
        syncing
            .entries(&[node])
            .into_iter()
            .map(|entry| entry.label)
            .collect()
    }

    #[test]
    fn an_unsynced_folder_can_only_be_synced() {
        let node = folder(1);
        assert_eq!(
            labels(&syncing(vec![], vec![]), &node),
            ["Sync to local folder…"]
        );
    }

    #[test]
    fn a_folder_synced_through_its_parent_can_be_left_out() {
        let node = folder(1);
        let syncing = syncing(vec![], vec![(node.clone(), Badge::Synced)]);
        assert_eq!(
            labels(&syncing, &node),
            ["Sync to local folder…", "Don't sync"]
        );
    }

    #[test]
    fn a_mapped_folder_can_be_opened_moved_or_stopped() {
        let node = folder(1);
        let mapping = Mapping {
            remote: Some(node.id),
            local: Some(PathBuf::from("/sync")),
        };
        assert_eq!(
            labels(&syncing(vec![mapping], vec![]), &node),
            ["Open local folder", "Change local folder…", "Stop syncing"]
        );
    }

    #[test]
    fn a_folder_set_to_not_sync_can_sync_again() {
        let node = folder(1);
        let mapping = Mapping {
            remote: Some(node.id),
            local: None,
        };
        assert_eq!(
            labels(&syncing(vec![mapping], vec![]), &node),
            ["Sync again"]
        );
    }

    #[test]
    fn several_selected_nodes_have_no_sync_entries() {
        let (a, b) = (folder(1), folder(2));
        assert!(syncing(vec![], vec![]).entries(&[&a, &b]).is_empty());
    }

    #[test]
    fn the_overall_state_is_the_worst_one() {
        let mapped = vec![Mapping {
            remote: None,
            local: Some(PathBuf::from("/sync")),
        }];
        let mut status = Status::default();
        assert_eq!(status.overall(), None);
        status.mappings = mapped;
        assert_eq!(status.overall(), Some(Badge::Synced));
        status.pending = 2;
        assert_eq!(status.overall(), Some(Badge::Pending));
        status.problems = vec![(PathBuf::from("/sync/a"), sync::Fault::now("local", "boom"))];
        assert_eq!(status.overall(), Some(Badge::Problem));
    }

    #[test]
    fn paths_in_the_home_folder_are_shortened() {
        let home = PathBuf::from(std::env::var_os("HOME").expect("HOME"));
        assert_eq!(display_path(&home.join("Sync/Photos")), "~/Sync/Photos");
        assert_eq!(display_path(&home), "~");
        assert_eq!(display_path(Path::new("/Volumes/Ext")), "/Volumes/Ext");
    }
}
