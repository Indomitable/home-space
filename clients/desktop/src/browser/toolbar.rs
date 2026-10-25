use iced::{
    Alignment, Element, Length,
    widget::{button, container, row, space, text, tooltip},
};

use protocol::{Node, NodeKind};

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
    Delete,
}

impl Action {
    fn icon(self) -> Icon {
        match self {
            Self::Open => Icon::Folder,
            Self::Download => Icon::Download,
            Self::Rename => Icon::Rename,
            Self::Move => Icon::Move,
            Self::Delete => Icon::Delete,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Open => "Open",
            Self::Download => "Download",
            Self::Rename => "Rename",
            Self::Move => "Move",
            Self::Delete => "Delete",
        }
    }

    fn shortcut(self) -> &'static str {
        match self {
            Self::Open | Self::Download => "Enter",
            Self::Rename => "F2",
            Self::Move => "",
            Self::Delete => "Delete",
        }
    }
}

/// The actions shown for `selected`, each with whether it is enabled.
pub fn actions(selected: &[&Node]) -> Vec<(Action, bool)> {
    let single = selected.len() == 1;
    let any = !selected.is_empty();
    let single_folder = matches!(selected, [node] if node.kind == NodeKind::Folder);
    let open = single_folder.then_some((Action::Open, true));
    open.into_iter()
        .chain([
            (Action::Download, any),
            (Action::Rename, single),
            (Action::Move, any),
            (Action::Delete, any),
        ])
        .collect()
}

/// The toolbar: folder-level commands, then actions on the selection.
pub fn view<'a>(selected: &[&Node]) -> Element<'a, Message> {
    let commands = row![
        tool(
            Icon::NewFolder,
            "New folder",
            "Ctrl+N",
            Some(Message::NewFolder),
            false
        ),
        tool(
            Icon::Upload,
            "Upload",
            "Ctrl+U",
            Some(Message::Upload),
            false
        ),
        tool(
            Icon::Refresh,
            "Refresh",
            "F5",
            Some(Message::Refresh),
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
            actions(selected)
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
