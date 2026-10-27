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
const BUTTON_PADDING: f32 = 10.0;
const CHAR_WIDTH: f32 = 8.5;
const MIN_GAP: f32 = 24.0;
const TIP_PADDING: f32 = 6.0;
const TIP_SIZE: f32 = 12.0;

#[cfg(target_os = "macos")]
const NEW_FOLDER_SHORTCUT: &str = "Cmd+N";
#[cfg(not(target_os = "macos"))]
const NEW_FOLDER_SHORTCUT: &str = "Ctrl+N";
#[cfg(target_os = "macos")]
const UPLOAD_SHORTCUT: &str = "Cmd+U";
#[cfg(not(target_os = "macos"))]
const UPLOAD_SHORTCUT: &str = "Ctrl+U";

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

/// A toolbar button before it is laid out.
struct Tool<'a> {
    icon: Icon,
    label: &'a str,
    shortcut: &'a str,
    message: Option<Message>,
    danger: bool,
}

impl<'a> Tool<'a> {
    fn new(icon: Icon, label: &'a str, shortcut: &'a str, message: Option<Message>) -> Self {
        Self {
            icon,
            label,
            shortcut,
            message,
            danger: false,
        }
    }

    fn action(action: Action, enabled: bool) -> Self {
        Self {
            danger: action == Action::Delete,
            ..Self::new(
                action.icon(),
                action.label(),
                action.shortcut(),
                enabled.then_some(Message::Act(action)),
            )
        }
    }

    /// Roughly how wide the button is, with or without its label.
    fn width(&self, labelled: bool) -> f32 {
        let label = if labelled {
            LABEL_SPACING + text_width(self.label)
        } else {
            0.0
        };
        BUTTON_PADDING * 2.0 + ICON_SIZE + label
    }
}

/// The toolbar, `width` wide. Without a selection it holds the folder commands; with
/// one it holds the actions on it instead. When the labels do not fit, the buttons
/// show only their icons and the label moves into the tooltip.
/// `can_add` is false where nothing may be created, such as Lost & Found.
pub fn view<'a>(
    selected: &[&Node],
    lost_and_found: NodeId,
    can_add: bool,
    width: f32,
) -> Element<'a, Message> {
    let (lead, leading, trailing) = if selected.is_empty() {
        (
            None,
            vec![
                Tool::new(
                    Icon::NewFolder,
                    "New folder",
                    NEW_FOLDER_SHORTCUT,
                    can_add.then_some(Message::NewFolder),
                ),
                Tool::new(
                    Icon::Upload,
                    "Upload",
                    UPLOAD_SHORTCUT,
                    can_add.then_some(Message::Upload),
                ),
                Tool::new(Icon::Refresh, "Refresh", "F5", Some(Message::Refresh)),
            ],
            vec![Tool::new(
                Icon::Recycling,
                "Recycle bin",
                "",
                Some(Message::OpenTrash),
            )],
        )
    } else {
        let (trailing, leading): (Vec<_>, Vec<_>) = actions(selected, lost_and_found)
            .into_iter()
            .filter(|(action, _)| *action != Action::Open)
            .map(|(action, enabled)| Tool::action(action, enabled))
            .partition(|tool| tool.danger);
        (Some(selected.len()), leading, trailing)
    };
    let labelled = fits(lead, leading.iter().chain(&trailing), width);

    let mut bar = row![].spacing(SPACING).align_y(Alignment::Center);
    if let Some(count) = lead {
        let clear = button(Icon::Close.view(ICON_SIZE, Tint::Muted))
            .style(theme::tool)
            .on_press(Message::ClearSelection);
        bar = bar.push(tip(clear.into(), "Clear selection", "Esc"));
        bar = bar.push(text(count_label(count, labelled)));
    }
    bar.extend(leading.into_iter().map(|tool| tool.view(labelled)))
        .push(space::horizontal())
        .extend(trailing.into_iter().map(|tool| tool.view(labelled)))
        .width(Length::Fill)
        .into()
}

/// Whether `tools`, after the selection count `lead`, fit in `width` with their labels.
fn fits<'t, 'a: 't>(
    lead: Option<usize>,
    tools: impl Iterator<Item = &'t Tool<'a>>,
    width: f32,
) -> bool {
    let lead = lead.map_or(0.0, |count| {
        BUTTON_PADDING * 2.0 + ICON_SIZE + SPACING + text_width(&count_label(count, true))
    });
    let (count, buttons) = tools.fold((0, 0.0), |(count, sum), tool| {
        (count + 1, sum + tool.width(true))
    });
    // One extra gap for the spacer between the leading and the trailing buttons.
    lead + buttons + SPACING * count as f32 + MIN_GAP <= width
}

fn count_label(count: usize, labelled: bool) -> String {
    if labelled {
        format!("{count} selected")
    } else {
        count.to_string()
    }
}

/// An estimate of how wide `label` renders at the default text size.
fn text_width(label: &str) -> f32 {
    label.chars().count() as f32 * CHAR_WIDTH
}

/// A menu entry for the context menu.
pub fn menu_item<'a>(action: Action, enabled: bool) -> Element<'a, Message> {
    let tint = match action {
        _ if !enabled => Tint::Disabled,
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

impl<'a> Tool<'a> {
    fn view(self, labelled: bool) -> Element<'a, Message> {
        let (style, tint): (fn(&iced::Theme, button::Status) -> button::Style, Tint) =
            if self.danger {
                (theme::danger_tool, Tint::Danger)
            } else {
                (theme::tool, Tint::Text)
            };
        let tint = if self.message.is_some() {
            tint
        } else {
            Tint::Disabled
        };
        let mut content = row![self.icon.view(ICON_SIZE, tint)]
            .spacing(LABEL_SPACING)
            .align_y(Alignment::Center);
        if labelled {
            content = content.push(text(self.label));
        }
        let tool = button(content).style(style).on_press_maybe(self.message);
        match (labelled, self.shortcut.is_empty()) {
            (true, true) => tool.into(),
            (true, false) => tip(tool.into(), "", self.shortcut),
            (false, _) => tip(tool.into(), self.label, self.shortcut),
        }
    }
}

/// `content` with a tooltip below it naming `label` and its `shortcut`.
fn tip<'a>(
    content: Element<'a, Message>,
    label: &'a str,
    shortcut: &'a str,
) -> Element<'a, Message> {
    let mut lines = row![].spacing(LABEL_SPACING).align_y(Alignment::Center);
    if !label.is_empty() {
        lines = lines.push(text(label).size(TIP_SIZE));
    }
    if !shortcut.is_empty() {
        lines = lines.push(text(shortcut).size(TIP_SIZE).style(text::secondary));
    }
    tooltip(
        content,
        container(lines).padding(TIP_PADDING).style(theme::panel),
        tooltip::Position::Bottom,
    )
    .into()
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

    fn selection_tools() -> Vec<Tool<'static>> {
        let (first, second) = (file(1), file(2));
        actions(&[&first, &second], LOST_AND_FOUND)
            .into_iter()
            .map(|(action, enabled)| Tool::action(action, enabled))
            .collect()
    }

    #[test]
    fn labels_show_in_a_wide_window() {
        assert!(fits(Some(2), selection_tools().iter(), 1000.0));
    }

    #[test]
    fn labels_drop_in_a_narrow_window() {
        assert!(!fits(Some(2), selection_tools().iter(), 450.0));
    }
}
