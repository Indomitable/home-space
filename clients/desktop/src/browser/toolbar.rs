use iced::{
    Alignment, Element, Length,
    widget::{button, container, row, space, text, tooltip},
};

use protocol::{Node, NodeId, NodeKind};

use super::Message;
use crate::{
    icons::{ICON_SIZE, Icon, Tint},
    theme,
};

const SPACING: f32 = 4.0;
const LABEL_SPACING: f32 = 6.0;
const TIP_PADDING: f32 = 6.0;
const TIP_SIZE: f32 = 12.0;

/// Something the user can do with the selected nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open,
    Download,
    Rename,
    Move,
    Details,
    Delete,
}

impl Action {
    fn icon(self) -> Icon {
        match self {
            Self::Open => Icon::Folder,
            Self::Download => Icon::Download,
            Self::Rename => Icon::Rename,
            Self::Move => Icon::Move,
            Self::Details => Icon::Info,
            Self::Delete => Icon::Delete,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Download => "Download",
            Self::Rename => "Rename",
            Self::Move => "Move",
            Self::Details => "Details",
            Self::Delete => "Delete",
        }
    }

    fn shortcut(self) -> &'static str {
        match self {
            Self::Open | Self::Download => "Enter",
            Self::Rename => "F2",
            Self::Move | Self::Details => "",
            Self::Delete => "Delete",
        }
    }
}

/// The actions shown for `selected`, each with whether it is enabled.
/// Lost & Found, identified by `lost_and_found`, cannot be deleted.
pub fn actions(selected: &[&Node], lost_and_found: NodeId) -> Vec<(Action, bool)> {
    let single = selected.len() == 1;
    let any = !selected.is_empty();
    let deletable = any && selected.iter().all(|node| node.id != lost_and_found);
    let single_folder = matches!(selected, [node] if node.kind == NodeKind::Folder);
    let open = single_folder.then_some((Action::Open, true));
    open.into_iter()
        .chain([
            (Action::Download, any),
            (Action::Rename, single),
            (Action::Move, any),
            (Action::Details, single),
            (Action::Delete, deletable),
        ])
        .collect()
}

/// The toolbar: folder-level commands, then actions on the selection.
/// `can_add` is false where nothing may be created, such as Lost & Found.
pub fn view<'a>(selected: &[&Node], lost_and_found: NodeId, can_add: bool) -> Element<'a, Message> {
    let commands = row![
        tool(
            Icon::NewFolder,
            "New folder",
            "Ctrl+N",
            can_add.then_some(Message::NewFolder),
            false
        ),
        tool(
            Icon::Upload,
            "Upload",
            "Ctrl+U",
            can_add.then_some(Message::Upload),
            false
        ),
        tool(
            Icon::Refresh,
            "Refresh",
            "F5",
            Some(Message::Refresh),
            false
        ),
        tool(
            Icon::Recycling,
            "Recycle bin",
            "",
            Some(Message::OpenTrash),
            false
        ),
    ]
    .spacing(SPACING);
    let mut bar = row![commands, space::horizontal()]
        .spacing(SPACING)
        .align_y(Alignment::Center);
    if !selected.is_empty() {
        bar = bar.push(text(format!("{} selected", selected.len())));
        bar = bar.extend(
            actions(selected, lost_and_found)
                .into_iter()
                .filter(|(action, _)| *action != Action::Open)
                .map(|(action, enabled)| {
                    tool(
                        action.icon(),
                        action.label(),
                        action.shortcut(),
                        enabled.then_some(Message::Act(action)),
                        action == Action::Delete,
                    )
                }),
        );
    }
    bar.width(Length::Fill).into()
}

/// A menu entry for the context menu.
pub fn menu_item<'a>(action: Action, enabled: bool) -> Element<'a, Message> {
    let tint = match action {
        Action::Delete => Tint::Danger,
        _ => Tint::Text,
    };
    let style = match action {
        Action::Delete => theme::danger_tool,
        _ => theme::tool,
    };
    button(
        row![
            action.icon().view(ICON_SIZE, tint),
            text(action.label()).width(Length::Fill),
            text(action.shortcut()).size(TIP_SIZE),
        ]
        .spacing(LABEL_SPACING * 2.0)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .style(style)
    .on_press_maybe(enabled.then_some(Message::Act(action)))
    .into()
}

fn tool<'a>(
    icon: Icon,
    label: &'a str,
    shortcut: &'a str,
    message: Option<Message>,
    danger: bool,
) -> Element<'a, Message> {
    let (style, tint): (fn(&iced::Theme, button::Status) -> button::Style, Tint) = if danger {
        (theme::danger_tool, Tint::Danger)
    } else {
        (theme::tool, Tint::Text)
    };
    let tool = button(
        row![icon.view(ICON_SIZE, tint), text(label)]
            .spacing(LABEL_SPACING)
            .align_y(Alignment::Center),
    )
    .style(style)
    .on_press_maybe(message);
    if shortcut.is_empty() {
        tool.into()
    } else {
        tooltip(
            tool,
            container(text(shortcut).size(TIP_SIZE))
                .padding(TIP_PADDING)
                .style(theme::panel),
            tooltip::Position::Bottom,
        )
        .into()
    }
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

    fn file(raw: u128) -> Node {
        Node {
            id: NodeId(Uuid::from_u128(raw)),
            parent_id: None,
            kind: NodeKind::File,
            name: format!("file-{raw}"),
            size: 0,
            sha256: None,
            mime: String::new(),
            version: 1,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    const LOST_AND_FOUND: NodeId = NodeId(Uuid::from_u128(99));

    fn delete_enabled(selected: &[&Node]) -> bool {
        actions(selected, LOST_AND_FOUND)
            .into_iter()
            .any(|(action, enabled)| action == Action::Delete && enabled)
    }

    fn details_enabled(selected: &[&Node]) -> bool {
        actions(selected, LOST_AND_FOUND)
            .into_iter()
            .any(|(action, enabled)| action == Action::Details && enabled)
    }

    #[test]
    fn details_are_available_for_a_single_node() {
        let node = file(1);
        assert!(details_enabled(&[&node]));
    }

    #[test]
    fn details_are_disabled_for_several_nodes() {
        let (first, second) = (file(1), file(2));
        assert!(!details_enabled(&[&first, &second]));
    }

    #[test]
    fn delete_is_available_for_ordinary_nodes() {
        let (first, second) = (file(1), file(2));
        assert!(delete_enabled(&[&first, &second]));
    }

    #[test]
    fn delete_is_disabled_for_lost_and_found() {
        let folder = file(99);
        assert!(!delete_enabled(&[&folder]));
    }

    #[test]
    fn delete_is_disabled_when_the_selection_holds_lost_and_found() {
        let (first, folder) = (file(1), file(99));
        assert!(!delete_enabled(&[&first, &folder]));
    }

    #[test]
    fn rename_and_move_stay_available_for_lost_and_found() {
        let folder = file(99);
        let enabled: Vec<_> = actions(&[&folder], LOST_AND_FOUND)
            .into_iter()
            .filter(|(_, enabled)| *enabled)
            .map(|(action, _)| action)
            .collect();
        assert!(enabled.contains(&Action::Rename) && enabled.contains(&Action::Move));
    }
}
