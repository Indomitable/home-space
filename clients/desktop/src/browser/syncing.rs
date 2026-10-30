use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, scrollable, space, text},
};

use protocol::{Node, NodeId, NodeKind};

use super::{HOME, Message};
use crate::{
    icons::{ICON_SIZE, Icon, Tint},
    sync::{self, Badge, Command, Commands, Mapping, Status},
    theme,
};

const PANEL_WIDTH: f32 = 560.0;
const PANEL_PADDING: f32 = 16.0;
const SPACING: f32 = 10.0;
const TITLE_SIZE: f32 = 18.0;
const NOTE_SIZE: f32 = 13.0;
const PROBLEMS_HEIGHT: f32 = 160.0;
const SHOWN_PROBLEMS: usize = 50;

/// What the browser knows of the sync engine.
#[derive(Debug, Default)]
pub struct Syncing {
    commands: Option<Commands>,
    status: Arc<Status>,
    /// Whether the sync settings are shown.
    pub settings: bool,
}

/// A context menu entry about syncing a folder.
pub struct Entry {
    pub icon: Icon,
    pub label: &'static str,
    pub message: Message,
}

impl Syncing {
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Takes in a report of the engine; returns why a mapping was refused, if it was.
    pub fn update(&mut self, update: sync::Update) -> Option<String> {
        match update {
            sync::Update::Ready(commands) => self.commands = Some(commands),
            sync::Update::Status(status) => self.status = status,
            sync::Update::Rejected(reason) => return Some(reason),
        }
        None
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
                    message: Message::SyncOpenLocal(local),
                },
                Entry {
                    icon: Icon::Sync,
                    label: "Change local folder…",
                    message: Message::SyncPick(id),
                },
                Entry {
                    icon: Icon::SyncDisabled,
                    label: "Stop syncing",
                    message: Message::SyncUnmap(id),
                },
            ],
            Some(None) => vec![Entry {
                icon: Icon::Sync,
                label: "Sync again",
                message: Message::SyncUnmap(id),
            }],
            None => {
                let mut entries = vec![Entry {
                    icon: Icon::Sync,
                    label: "Sync to local folder…",
                    message: Message::SyncPick(id),
                }];
                // Only a folder synced through a parent can be left out of it.
                if self.status.badge(id).is_some() {
                    entries.push(Entry {
                        icon: Icon::SyncDisabled,
                        label: "Don't sync",
                        message: Message::SyncSet(Mapping {
                            remote: id,
                            local: None,
                        }),
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
pub fn indicator(status: &Status) -> Element<'_, Message> {
    let (icon, tint, label) = match status.overall() {
        None => (Icon::Sync, Tint::Muted, "Sync".to_owned()),
        Some(Badge::Synced) => (Icon::Synced, Tint::Success, "Up to date".to_owned()),
        Some(Badge::Pending) => (
            Icon::SyncPending,
            Tint::Accent,
            format!("Syncing {}", status.pending),
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
    .on_press(Message::SyncSettings(true))
    .into()
}

/// The sync settings: what syncs where, and what went wrong.
pub fn settings(status: &Status) -> Element<'_, Message> {
    let close = button(Icon::Close.view(ICON_SIZE, Tint::Muted))
        .style(theme::tool)
        .on_press(Message::SyncSettings(false));
    let mut panel = column![
        row![text("Sync").size(TITLE_SIZE), space::horizontal(), close].align_y(Alignment::Center)
    ]
    .spacing(SPACING);
    if let Some(error) = &status.error {
        panel = panel.push(container(text(error.as_str())).style(theme::error));
    }
    if status.mapping(None).is_none() {
        panel = panel.push(mapping_row(HOME.to_owned(), None, None));
    }
    let mut mappings: Vec<&Mapping> = status.mappings.iter().collect();
    mappings.sort_by_key(|mapping| mapping.remote.map(|id| folder_name(status, id)));
    for mapping in mappings {
        let name = mapping
            .remote
            .map_or_else(|| HOME.to_owned(), |id| folder_name(status, id));
        panel = panel.push(mapping_row(name, Some(mapping), mapping.remote));
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
            .map(|(path, reason)| {
                text(format!("{}: {reason}", path.display()))
                    .size(NOTE_SIZE)
                    .into()
            });
        panel = panel
            .push(text(format!(
                "{} could not be synced",
                plural(status.problems.len())
            )))
            .push(scrollable(column(problems).spacing(SPACING / 2.0)).height(PROBLEMS_HEIGHT));
    }
    container(panel)
        .padding(PANEL_PADDING)
        .width(PANEL_WIDTH)
        .style(theme::panel)
        .into()
}

/// A mapped folder (or Home, unmapped) with what can be done with it.
fn mapping_row<'a>(
    name: String,
    mapping: Option<&'a Mapping>,
    remote: Option<NodeId>,
) -> Element<'a, Message> {
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
                Message::SyncPick(remote),
            ));
        }
        Some(mapping) => {
            if let Some(local) = &mapping.local {
                actions = actions.push(tool(
                    Icon::FolderOpen,
                    "Open",
                    Message::SyncOpenLocal(local.clone()),
                ));
            }
            actions = actions.push(tool(Icon::Close, "Remove", Message::SyncUnmap(remote)));
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

fn tool(icon: Icon, label: &str, message: Message) -> Element<'_, Message> {
    button(
        row![icon.view(ICON_SIZE, Tint::Text), text(label)]
            .spacing(SPACING / 2.0)
            .align_y(Alignment::Center),
    )
    .style(theme::tool)
    .on_press(message)
    .into()
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
    #[cfg(target_os = "macos")]
    const OPENER: &str = "open";
    #[cfg(target_os = "windows")]
    const OPENER: &str = "explorer";
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    const OPENER: &str = "xdg-open";
    std::process::Command::new(OPENER)
        .arg(path)
        .spawn()
        .map(drop)
        .map_err(|err| format!("could not open {}: {err}", path.display()))
}

/// Asks for the local folder to sync with.
pub async fn pick_folder() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .set_title("Choose the local folder to sync with")
        .pick_folder()
        .await
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
        status.problems = vec![(PathBuf::from("/sync/a"), "boom".to_owned())];
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
