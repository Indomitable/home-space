use std::collections::HashSet;

use client_core::{
    format::human_size,
    listing::{Sort, SortKey},
};
use iced::widget::text::LineHeight;
use iced::{
    Alignment, Element, Length, Padding,
    alignment::Horizontal,
    widget::{button, column, container, row, scrollable, space, stack, text},
};
use protocol::{Node, NodeId, NodeKind};

use super::{
    Message,
    anchor::Anchor,
    context_menu::Placement,
    selection::{Coverage, Selection},
    syncing::{badge_icon, display_path},
};
use crate::{
    ellipsis::ellipsized,
    icons::{ICON_SIZE, Icon, Tint, node_icon},
    locale::short_date,
    sync::{Badge, Item, Status},
    theme,
};

pub(super) const CHECK_WIDTH: f32 = 24.0;
pub(super) const SIZE_WIDTH: f32 = 96.0;
pub(super) const DATE_WIDTH: f32 = 140.0;
pub(super) const CELL_SPACING: f32 = 12.0;
pub(super) const ROW_PADDING: Padding = Padding {
    top: 6.0,
    right: 12.0,
    bottom: 6.0,
    left: 8.0,
};
pub(super) const SCROLLBAR_GUTTER: f32 = 14.0;
pub(super) const ROW_GAP: f32 = 2.0;
pub(super) const HEADER_SIZE: f32 = 13.0;
const SORT_ARROW_GAP: f32 = 8.0;
pub(super) const ROW_SIZE: f32 = 14.0;
pub(super) const EMPTY_PADDING: f32 = 32.0;
const BADGE_SIZE: f32 = 12.0;
const LOCATION_SIZE: f32 = 12.0;
const BADGE_PADDING: f32 = 1.0;
/// The icon cell, leaving room for the badge to sit on the icon's corner.
const ICON_BOX: f32 = ICON_SIZE + 6.0;

/// Column titles with the select-all checkbox, aligned with [`rows`].
pub fn header<'a>(coverage: Coverage, sort: Sort) -> Element<'a, Message> {
    let icon = match coverage {
        Coverage::None => Icon::Unchecked,
        Coverage::Some => Icon::Mixed,
        Coverage::All => Icon::Checked,
    };
    let cells = row![
        check(icon, Message::ToggleAll),
        sort_button("Name", SortKey::Name, sort, Length::Fill, Alignment::Start),
        sort_button(
            "Size",
            SortKey::Size,
            sort,
            SIZE_WIDTH.into(),
            Alignment::End
        ),
        sort_button(
            "Modified",
            SortKey::Modified,
            sort,
            DATE_WIDTH.into(),
            Alignment::Start
        ),
        space().width(CHECK_WIDTH),
    ]
    .spacing(CELL_SPACING)
    .align_y(Alignment::Center);
    container(container(cells).padding(ROW_PADDING).style(theme::header))
        .padding(Padding::default().right(SCROLLBAR_GUTTER))
        .into()
}

/// A column title that sorts by `key` when pressed, with an arrow while it is active.
fn sort_button(
    title: &str,
    key: SortKey,
    sort: Sort,
    width: Length,
    align: Alignment,
) -> Element<'_, Message> {
    let arrow = match (sort.key == key, sort.descending) {
        (false, _) => None,
        (true, false) => Some("\u{2191}"),
        (true, true) => Some("\u{2193}"),
    };
    let label = row![
        text(title).size(HEADER_SIZE).align_y(Alignment::Center),
        arrow.map(|a| text(a)
            .align_y(Alignment::Center)
            .size(20.0)
            .line_height(LineHeight::Relative(1.1))
            .height(25.0)
            .font(theme::BOLD))
    ]
    .spacing(SORT_ARROW_GAP)
    .align_y(Alignment::Center);
    button(container(label).width(Length::Fill).align_x(align))
        .padding(0)
        .width(width)
        .style(button::text)
        .on_press(Message::SortBy(key))
        .into()
}

/// The scrolling list of rows, or a hint when the folder is empty. After the listed `nodes`
/// come the new files of the `folder` that the sync has yet to upload, which are not on the
/// server to be listed.
pub fn rows<'a>(
    nodes: &'a [Node],
    folder: Option<NodeId>,
    selection: &Selection,
    sync: &'a Status,
) -> Element<'a, Message> {
    let listed: HashSet<&str> = nodes.iter().map(|node| node.name.as_str()).collect();
    // A file that just landed is listed before the sync reports it done.
    let mut waiting: Vec<&Item> = sync
        .new_uploads_in(folder)
        .filter(|item| !listed.contains(item.name.as_str()))
        .collect();
    waiting.sort_by_cached_key(|item| item.name.to_lowercase());
    if nodes.is_empty() && waiting.is_empty() {
        container(text("Nothing here yet. Drop files anywhere to upload."))
            .padding(EMPTY_PADDING)
            .center_x(Length::Fill)
            .into()
    } else {
        let rows = nodes
            .iter()
            .map(|node| {
                let location = sync.location(Some(node.id)).map(display_path);
                node_row(
                    node,
                    selection.contains(node.id),
                    sync.badge(Some(node.id)),
                    location,
                )
            })
            .chain(waiting.into_iter().map(pending_row));
        scrollable(
            column(rows)
                .spacing(ROW_GAP)
                .padding(Padding::default().right(SCROLLBAR_GUTTER)),
        )
        .height(Length::Fill)
        .into()
    }
}

fn node_row(
    node: &Node,
    selected: bool,
    badge: Option<Badge>,
    location: Option<String>,
) -> Element<'_, Message> {
    let icon = node_icon(node);
    let tint = match icon {
        Icon::Folder => Tint::Accent,
        _ => Tint::Muted,
    };
    let size = match node.kind {
        NodeKind::Folder => String::new(),
        NodeKind::File => human_size(node.size),
    };
    let check_icon = if selected {
        Icon::Checked
    } else {
        Icon::Unchecked
    };
    let cells = row![
        check(check_icon, Message::Toggle(node.id)),
        row![
            badged(icon.view(ICON_SIZE, tint), badge),
            name_cell(node.name.as_str(), location)
        ]
        .spacing(CELL_SPACING / 1.5)
        .align_y(Alignment::Center)
        .width(Length::Fill),
        ellipsized(size, ROW_SIZE, Alignment::End).width(SIZE_WIDTH),
        ellipsized(short_date(node.updated_at), ROW_SIZE, Alignment::Start).width(DATE_WIDTH),
        menu_button(node.id),
    ]
    .spacing(CELL_SPACING)
    .align_y(Alignment::Center);
    let row_button = button(cells)
        .width(Length::Fill)
        .padding(ROW_PADDING)
        .style(theme::row(selected))
        .on_press(Message::Click(node.id));
    let id = node.id;
    Anchor::new(row_button)
        .on_right_press(move |cursor| Message::ContextMenu(id, Placement::cursor(cursor)))
        .into()
}

/// A new file or folder waiting to be uploaded: grayed out and inert, with no menu to act on
/// what is not on the server yet.
fn pending_row(item: &Item) -> Element<'_, Message> {
    let icon = if item.is_folder() {
        Icon::Folder
    } else {
        Icon::Other
    };
    let size = if item.is_folder() {
        String::new()
    } else {
        human_size(i64::try_from(item.size).unwrap_or(i64::MAX))
    };
    let waiting = row![
        Icon::Update.view(ICON_SIZE, Tint::Muted),
        text("pending sync").size(ROW_SIZE)
    ]
    .spacing(CELL_SPACING / 2.0)
    .align_y(Alignment::Center);
    let cells = row![
        space().width(CHECK_WIDTH),
        row![
            container(icon.view(ICON_SIZE, Tint::Disabled)).width(ICON_BOX),
            ellipsized(item.name.as_str(), ROW_SIZE, Alignment::Start).width(Length::Fill)
        ]
        .spacing(CELL_SPACING / 1.5)
        .align_y(Alignment::Center)
        .width(Length::Fill),
        ellipsized(size, ROW_SIZE, Alignment::End).width(SIZE_WIDTH),
        container(waiting).width(DATE_WIDTH + CELL_SPACING + CHECK_WIDTH),
    ]
    .spacing(CELL_SPACING)
    .align_y(Alignment::Center);
    container(cells)
        .width(Length::Fill)
        .padding(ROW_PADDING)
        .style(theme::disabled)
        .into()
}

/// The three dots that open the context menu for the row, like right-clicking it.
fn menu_button<'a>(id: NodeId) -> Element<'a, Message> {
    let button = button(Icon::More.view(ICON_SIZE, Tint::Muted))
        .padding(0)
        .width(CHECK_WIDTH)
        .style(button::text)
        // Never sent: the anchor takes the press, and only needs the button to look enabled.
        .on_press(Message::CloseMenu);
    Anchor::new(button)
        .on_press(move |bounds| {
            Message::ContextMenu(id, Placement::hanging(bounds, Horizontal::Right))
        })
        .into()
}

/// The name, with the local folder it syncs to underneath when it has one.
fn name_cell(name: &str, location: Option<String>) -> Element<'_, Message> {
    let name = ellipsized(name, ROW_SIZE, Alignment::Start).width(Length::Fill);
    match location {
        Some(location) => column![
            name,
            container(ellipsized(location, LOCATION_SIZE, Alignment::Start).width(Length::Fill))
                .style(theme::muted)
        ]
        .width(Length::Fill)
        .into(),
        None => name.into(),
    }
}

/// `icon` with the sync `badge` on its lower right corner.
fn badged<'a>(icon: Element<'a, Message>, badge: Option<Badge>) -> Element<'a, Message> {
    let mut layers = vec![
        container(icon)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Alignment::Center)
            .into(),
    ];
    if let Some(badge) = badge {
        let (badge, tint) = badge_icon(badge);
        let disc = container(badge.view(BADGE_SIZE, tint))
            .padding(BADGE_PADDING)
            .style(theme::badge);
        layers.push(
            container(disc)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_right(Length::Fill)
                .align_bottom(Length::Fill)
                .into(),
        );
    }
    stack(layers).width(ICON_BOX).height(ICON_BOX).into()
}

pub(super) fn check<'a, M: Clone + 'a>(icon: Icon, message: M) -> Element<'a, M> {
    let tint = match icon {
        Icon::Unchecked => Tint::Muted,
        _ => Tint::Accent,
    };
    button(icon.view(ICON_SIZE, tint))
        .padding(0)
        .width(CHECK_WIDTH)
        .style(button::text)
        .on_press(message)
        .into()
}
