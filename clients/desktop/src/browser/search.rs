use std::time::Duration;

use iced::{
    Alignment, Element, Length, Padding,
    widget::{button, column, container, row, scrollable, text},
};

use api_client::{ApiError, format::human_size};
use protocol::{NodeId, NodeKind, SearchHit};

use super::{
    Message,
    grid::{
        CELL_SPACING, DATE_WIDTH, ROW_GAP, ROW_PADDING, ROW_SIZE, SCROLLBAR_GUTTER, SIZE_WIDTH,
    },
};
use crate::{
    icons::{ICON_SIZE, Icon, Tint, node_icon},
    locale::short_date,
    theme,
};

/// How long typing must pause before the query is sent.
pub const DEBOUNCE: Duration = Duration::from_millis(250);

const HOME: &str = "Home";
const LOCATION_SEPARATOR: &str = " / ";
const HEADER_SIZE: f32 = 13.0;
const EMPTY_PADDING: f32 = 32.0;
const NAME_PORTION: u16 = 3;
const LOCATION_PORTION: u16 = 2;

/// The search box and the results of the last query that finished.
#[derive(Debug, Default)]
pub struct Search {
    query: String,
    pub(super) seq: u64,
    hits: Vec<SearchHit>,
    loading: bool,
    error: Option<String>,
}

impl Search {
    /// Whether results replace the folder listing.
    pub fn is_active(&self) -> bool {
        !self.query.trim().is_empty()
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// Takes the text typed so far; returns the sequence number to debounce when there is
    /// something to search for.
    pub fn change(&mut self, query: String) -> Option<u64> {
        self.query = query;
        self.seq += 1;
        self.hits.clear();
        self.error = None;
        self.loading = false;
        self.is_active().then_some(self.seq)
    }

    pub fn clear(&mut self) {
        self.change(String::new());
    }

    /// Whether `seq` still belongs to the latest typed query.
    pub fn accepts(&self, seq: u64) -> bool {
        seq == self.seq
    }

    pub fn start(&mut self) {
        self.loading = true;
        self.error = None;
    }

    pub fn finish(&mut self, result: Result<Vec<SearchHit>, ApiError>) {
        self.loading = false;
        match result {
            Ok(hits) => self.hits = hits,
            Err(err) => self.error = Some(err.to_string()),
        }
    }

    pub fn hit(&self, index: usize) -> Option<&SearchHit> {
        self.hits.get(index)
    }

    /// Column titles and the scrolling result rows, or a hint when there is nothing to list.
    pub fn view(&self) -> Element<'_, Message> {
        if let Some(error) = &self.error {
            container(text(error.as_str())).style(theme::error).into()
        } else if self.hits.is_empty() {
            let hint = if self.loading {
                "Searching\u{2026}"
            } else {
                "Nothing matches your search."
            };
            container(text(hint))
                .padding(EMPTY_PADDING)
                .center_x(Length::Fill)
                .into()
        } else {
            let rows = self
                .hits
                .iter()
                .enumerate()
                .map(|(index, hit)| hit_row(index, hit));
            column![
                header(),
                scrollable(
                    column(rows)
                        .spacing(ROW_GAP)
                        .padding(Padding::default().right(SCROLLBAR_GUTTER)),
                )
                .height(Length::Fill),
            ]
            .spacing(ROW_GAP)
            .into()
        }
    }
}

/// The folder to open for a hit: the folder itself, or the folder holding a file.
pub fn destination(hit: &SearchHit) -> Option<NodeId> {
    match hit.node.kind {
        NodeKind::Folder => Some(hit.node.id),
        NodeKind::File => hit.node.parent_id,
    }
}

fn location_label(location: &[String]) -> String {
    std::iter::once(HOME)
        .chain(location.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(LOCATION_SEPARATOR)
}

fn header<'a>() -> Element<'a, Message> {
    let title = |label: &'static str| text(label).size(HEADER_SIZE);
    let cells = row![
        title("Name").width(Length::FillPortion(NAME_PORTION)),
        title("Location").width(Length::FillPortion(LOCATION_PORTION)),
        title("Size").width(SIZE_WIDTH).align_x(Alignment::End),
        title("Modified").width(DATE_WIDTH),
    ]
    .spacing(CELL_SPACING)
    .align_y(Alignment::Center);
    container(container(cells).padding(ROW_PADDING).style(theme::header))
        .padding(Padding::default().right(SCROLLBAR_GUTTER))
        .into()
}

fn hit_row(index: usize, hit: &SearchHit) -> Element<'_, Message> {
    let node = &hit.node;
    let icon = node_icon(node);
    let tint = match icon {
        Icon::Folder => Tint::Accent,
        _ => Tint::Muted,
    };
    let size = match node.kind {
        NodeKind::Folder => String::new(),
        NodeKind::File => human_size(node.size),
    };
    let cells = row![
        row![
            icon.view(ICON_SIZE, tint),
            text(node.name.as_str()).size(ROW_SIZE)
        ]
        .spacing(CELL_SPACING / 1.5)
        .align_y(Alignment::Center)
        .width(Length::FillPortion(NAME_PORTION)),
        text(location_label(&hit.location))
            .size(ROW_SIZE)
            .width(Length::FillPortion(LOCATION_PORTION)),
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
    button(cells)
        .width(Length::Fill)
        .padding(ROW_PADDING)
        .style(theme::row(false))
        .on_press(Message::SearchOpen(index))
        .into()
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;
    use uuid::Uuid;

    use super::*;

    fn hit(kind: NodeKind, parent: Option<u128>) -> SearchHit {
        SearchHit {
            node: protocol::Node {
                id: NodeId(Uuid::from_u128(1)),
                parent_id: parent.map(|raw| NodeId(Uuid::from_u128(raw))),
                kind,
                name: "n".to_owned(),
                size: 0,
                sha256: None,
                mime: None,
                created_at: OffsetDateTime::UNIX_EPOCH,
                updated_at: OffsetDateTime::UNIX_EPOCH,
            },
            location: Vec::new(),
        }
    }

    #[test]
    fn a_folder_hit_opens_the_folder_itself() {
        let hit = hit(NodeKind::Folder, Some(7));
        assert_eq!(destination(&hit), Some(hit.node.id));
    }

    #[test]
    fn a_file_hit_opens_its_parent() {
        assert_eq!(
            destination(&hit(NodeKind::File, Some(7))),
            Some(NodeId(Uuid::from_u128(7)))
        );
    }

    #[test]
    fn a_root_file_hit_opens_the_root() {
        assert_eq!(destination(&hit(NodeKind::File, None)), None);
    }

    #[test]
    fn locations_start_at_home() {
        assert_eq!(location_label(&[]), "Home");
        assert_eq!(
            location_label(&["a".to_owned(), "b".to_owned()]),
            "Home / a / b"
        );
    }

    #[test]
    fn blank_text_is_not_a_search() {
        let mut search = Search::default();
        assert_eq!(search.change("  ".to_owned()), None);
        assert!(!search.is_active());
    }

    #[test]
    fn typing_invalidates_earlier_queries() {
        let mut search = Search::default();
        let first = search.change("a".to_owned()).expect("active");
        let second = search.change("ab".to_owned()).expect("active");
        assert!(!search.accepts(first));
        assert!(search.accepts(second));
    }

    #[test]
    fn clearing_ends_the_search() {
        let mut search = Search::default();
        search.change("a".to_owned());
        search.clear();
        assert!(!search.is_active());
        assert_eq!(search.query(), "");
    }
}
