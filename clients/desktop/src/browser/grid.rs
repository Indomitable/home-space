use api_client::{
    format::human_size,
    listing::{Sort, SortKey},
};
use iced::font::Weight;
use iced::widget::text::LineHeight;
use iced::{
    Alignment, Element, Font, Length, Padding,
    widget::{button, column, container, mouse_area, row, scrollable, text},
};
use protocol::{Node, NodeKind};

use super::{
    Message,
    selection::{Coverage, Selection},
};
use crate::{
    icons::{ICON_SIZE, Icon, Tint, node_icon},
    locale::short_date,
    theme,
};

const CHECK_WIDTH: f32 = 24.0;
const SIZE_WIDTH: f32 = 96.0;
const DATE_WIDTH: f32 = 140.0;
const CELL_SPACING: f32 = 12.0;
const ROW_PADDING: Padding = Padding {
    top: 6.0,
    right: 12.0,
    bottom: 6.0,
    left: 8.0,
};
const SCROLLBAR_GUTTER: f32 = 14.0;
const ROW_GAP: f32 = 2.0;
const HEADER_SIZE: f32 = 13.0;
const SORT_ARROW_GAP: f32 = 8.0;
const ROW_SIZE: f32 = 15.0;
const EMPTY_PADDING: f32 = 32.0;

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
    let bold = Font {
        weight: Weight::Bold,
        ..Font::DEFAULT
    };
    let label = row![
        text(title).size(HEADER_SIZE).align_y(Alignment::Center),
        arrow.map(|a| text(a)
            .align_y(Alignment::Center)
            .size(20.0)
            .line_height(LineHeight::Relative(1.1))
            .height(25.0)
            .font(bold))
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

/// The scrolling list of rows, or a hint when the folder is empty.
pub fn rows<'a>(nodes: &'a [Node], selection: &Selection) -> Element<'a, Message> {
    if nodes.is_empty() {
        container(text("Nothing here yet. Drop files anywhere to upload."))
            .padding(EMPTY_PADDING)
            .center_x(Length::Fill)
            .into()
    } else {
        let rows = nodes
            .iter()
            .map(|node| node_row(node, selection.contains(node.id)));
        scrollable(
            column(rows)
                .spacing(ROW_GAP)
                .padding(Padding::default().right(SCROLLBAR_GUTTER)),
        )
        .height(Length::Fill)
        .into()
    }
}

fn node_row(node: &Node, selected: bool) -> Element<'_, Message> {
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
            icon.view(ICON_SIZE, tint),
            text(node.name.as_str()).size(ROW_SIZE)
        ]
        .spacing(CELL_SPACING / 1.5)
        .align_y(Alignment::Center)
        .width(Length::Fill),
        text(size)
            .size(ROW_SIZE)
            .width(SIZE_WIDTH)
            .align_x(Alignment::End),
        text(short_date(node.updated_at))
            .size(ROW_SIZE)
            .width(DATE_WIDTH),
    ]
    .spacing(CELL_SPACING)
    .align_y(Alignment::Center);
    let row_button = button(cells)
        .width(Length::Fill)
        .padding(ROW_PADDING)
        .style(theme::row(selected))
        .on_press(Message::Click(node.id));
    mouse_area(row_button)
        .on_right_press(Message::ContextMenu(node.id))
        .into()
}

fn check<'a>(icon: Icon, message: Message) -> Element<'a, Message> {
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
