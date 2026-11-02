use iced::{
    Alignment, Element, Length, Point, Rectangle, Size,
    alignment::Horizontal,
    widget::{button, column, container, row, text},
};

use protocol::{Node, NodeId, NodeKind};

use super::{
    Message,
    toolbar::{actions, menu_item, menu_row},
};
use crate::{
    icons::{ICON_SIZE, Icon, Tint},
    theme,
};

const MENU_WIDTH: f32 = 220.0;
/// Fixed, so the menu's height is known before it is laid out and it can hang above a widget.
pub(super) const ITEM_HEIGHT: f32 = 36.0;
const MENU_PADDING: f32 = 4.0;
const EDGE_MARGIN: f32 = 8.0;
const LABEL_SPACING: f32 = 12.0;

/// An extra entry of the context menu, below the actions.
pub struct Entry {
    pub icon: Icon,
    pub label: &'static str,
    pub message: Message,
}

/// The entry opening the selected folder in the web client: only a single folder has one.
pub fn web_entry(selected: &[&Node]) -> Option<Entry> {
    match selected {
        [node] if node.kind == NodeKind::Folder => Some(Entry {
            icon: Icon::Web,
            label: "Open in web",
            message: Message::OpenInWeb(Some(node.id)),
        }),
        _ => None,
    }
}

/// The context menu for `selected`, with the web entry after Open, followed by the sync `entries`.
pub fn view<'a>(
    selected: &[&Node],
    lost_and_found: NodeId,
    entries: Vec<Entry>,
) -> Element<'a, Message> {
    let mut items: Vec<_> = actions(selected, lost_and_found)
        .into_iter()
        .map(|(action, enabled)| menu_item(action, enabled))
        .collect();
    // Only a single folder has a web entry, and Open is then the first item.
    if let Some(web) = web_entry(selected) {
        items.insert(1, entry(web));
    }
    panel(items.into_iter().chain(entries.into_iter().map(entry)))
}

fn entry<'a>(entry: Entry) -> Element<'a, Message> {
    menu_row(
        entry.icon,
        Tint::Text,
        entry.label,
        "",
        theme::tool,
        Some(entry.message),
    )
}

/// A menu of `folders` that opens the chosen one.
pub fn folders(folders: &[Node]) -> Element<'_, Message> {
    let items = folders.iter().map(|folder| {
        button(
            row![
                Icon::Folder.view(ICON_SIZE, Tint::Accent),
                text(folder.name.as_str())
            ]
            .spacing(LABEL_SPACING)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(ITEM_HEIGHT)
        .style(theme::tool)
        .on_press(Message::Jump(Some(folder.id)))
        .into()
    });
    panel(items)
}

fn panel<'a>(items: impl IntoIterator<Item = Element<'a, Message>>) -> Element<'a, Message> {
    container(column(items))
        .padding(MENU_PADDING)
        .width(MENU_WIDTH)
        .style(theme::panel)
        .into()
}

/// What a menu hangs from: the pointer, or a widget whose `edge` the menu lines up with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    target: Rectangle,
    edge: Horizontal,
}

impl Placement {
    /// A menu opening under the pointer at `cursor`.
    pub fn cursor(cursor: Point) -> Self {
        Self {
            target: Rectangle::new(cursor, Size::ZERO),
            edge: Horizontal::Left,
        }
    }

    /// A menu hanging from `target`: below it, or above when there is no room, with its `edge`
    /// lined up with the same edge of `target`.
    pub fn hanging(target: Rectangle, edge: Horizontal) -> Self {
        Self { target, edge }
    }
}

/// Where to put the menu so it stays inside `window`.
pub fn position(placement: Placement, window: Size, items: usize) -> Point {
    let Placement { target, edge } = placement;
    let height = ITEM_HEIGHT * items as f32 + MENU_PADDING * 2.0;
    let max_x = (window.width - MENU_WIDTH - EDGE_MARGIN).max(0.0);
    let max_y = (window.height - height - EDGE_MARGIN).max(0.0);
    let x = match edge {
        Horizontal::Right => target.x + target.width - MENU_WIDTH,
        _ => target.x,
    };
    let below = target.y + target.height;
    let y = if below <= max_y {
        below
    } else {
        target.y - height
    };
    Point::new(x.clamp(0.0, max_x), y.clamp(0.0, max_y))
}

/// How many entries the menu shows for `selected`.
pub fn item_count(selected: &[&Node], lost_and_found: NodeId) -> usize {
    actions(selected, lost_and_found).len() + usize::from(web_entry(selected).is_some())
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

    fn node(kind: NodeKind) -> Node {
        Node {
            id: NodeId(Uuid::from_u128(1)),
            parent_id: None,
            kind,
            name: "node".to_owned(),
            size: 0,
            sha256: None,
            mime: String::new(),
            version: 1,
            created_at: OffsetDateTime::UNIX_EPOCH,
            updated_at: OffsetDateTime::UNIX_EPOCH,
        }
    }

    #[test]
    fn only_a_single_folder_opens_in_web() {
        let (folder, file) = (node(NodeKind::Folder), node(NodeKind::File));
        assert!(web_entry(&[&folder]).is_some());
        assert!(web_entry(&[&file]).is_none());
        assert!(web_entry(&[&folder, &folder]).is_none());
        assert!(web_entry(&[]).is_none());
    }

    #[test]
    fn menu_opens_at_the_cursor_when_it_fits() {
        let placement = Placement::cursor(Point::new(100.0, 100.0));
        let point = position(placement, Size::new(1000.0, 800.0), 4);
        assert_eq!(point, Point::new(100.0, 100.0));
    }

    #[test]
    fn menu_is_pushed_back_inside_the_window() {
        let placement = Placement::cursor(Point::new(990.0, 790.0));
        let point = position(placement, Size::new(1000.0, 800.0), 4);
        assert!(point.x + MENU_WIDTH <= 1000.0);
        assert!(point.y + ITEM_HEIGHT * 4.0 <= 800.0);
    }

    #[test]
    fn menu_hangs_below_the_target_with_the_same_edge() {
        let target = Rectangle::new(Point::new(400.0, 100.0), Size::new(24.0, 20.0));
        let window = Size::new(1000.0, 800.0);

        let left = position(Placement::hanging(target, Horizontal::Left), window, 4);
        let right = position(Placement::hanging(target, Horizontal::Right), window, 4);

        assert_eq!(left, Point::new(400.0, 120.0));
        assert_eq!(right, Point::new(424.0 - MENU_WIDTH, 120.0));
    }

    #[test]
    fn menu_hangs_above_the_target_when_there_is_no_room_below() {
        let target = Rectangle::new(Point::new(400.0, 700.0), Size::new(24.0, 20.0));
        let point = position(
            Placement::hanging(target, Horizontal::Right),
            Size::new(1000.0, 800.0),
            4,
        );
        let height = ITEM_HEIGHT * 4.0 + MENU_PADDING * 2.0;
        assert_eq!(point.y, 700.0 - height);
    }
}
