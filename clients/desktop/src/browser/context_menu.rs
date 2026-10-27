use iced::{
    Alignment, Element, Length, Point, Size,
    widget::{button, column, container, row, text},
};

use protocol::{Node, NodeId};

use super::{
    Message,
    toolbar::{actions, menu_item},
};
use crate::{
    icons::{ICON_SIZE, Icon, Tint},
    theme,
};

const MENU_WIDTH: f32 = 220.0;
const ITEM_HEIGHT: f32 = 36.0;
const MENU_PADDING: f32 = 4.0;
const EDGE_MARGIN: f32 = 8.0;
const LABEL_SPACING: f32 = 12.0;

/// The context menu for `selected`.
pub fn view<'a>(selected: &[&Node], lost_and_found: NodeId) -> Element<'a, Message> {
    let items = actions(selected, lost_and_found)
        .into_iter()
        .map(|(action, enabled)| menu_item(action, enabled));
    panel(items)
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
        .style(theme::tool)
        .on_press(Message::Open(Some(folder.id)))
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

/// Where to put the menu opened at `cursor` so it stays inside `window`.
pub fn position(cursor: Point, window: Size, items: usize) -> Point {
    let height = ITEM_HEIGHT * items as f32 + MENU_PADDING * 2.0;
    let max_x = (window.width - MENU_WIDTH - EDGE_MARGIN).max(0.0);
    let max_y = (window.height - height - EDGE_MARGIN).max(0.0);
    Point::new(cursor.x.min(max_x), cursor.y.min(max_y))
}

/// How many entries the menu shows for `selected`.
pub fn item_count(selected: &[&Node], lost_and_found: NodeId) -> usize {
    actions(selected, lost_and_found).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_opens_at_the_cursor_when_it_fits() {
        let point = position(Point::new(100.0, 100.0), Size::new(1000.0, 800.0), 4);
        assert_eq!(point, Point::new(100.0, 100.0));
    }

    #[test]
    fn menu_is_pushed_back_inside_the_window() {
        let point = position(Point::new(990.0, 790.0), Size::new(1000.0, 800.0), 4);
        assert!(point.x + MENU_WIDTH <= 1000.0);
        assert!(point.y + ITEM_HEIGHT * 4.0 <= 800.0);
    }
}
