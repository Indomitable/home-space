use std::time::Duration;

use iced::{
    Alignment, Element, Length, Padding, Task,
    widget::{button, column, container, row, scrollable, text},
};

use api_client::{ApiError, Client};
use client_core::{format::human_size, labels::location_label, search::destination};
use protocol::{NodeId, NodeKind, SearchHit};

use super::{
    Message as BrowserMessage,
    grid::{
        CELL_SPACING, DATE_WIDTH, ROW_GAP, ROW_PADDING, ROW_SIZE, SCROLLBAR_GUTTER, SIZE_WIDTH,
    },
};
use crate::{
    ellipsis::ellipsized,
    icons::{ICON_SIZE, Icon, Tint, node_icon},
    locale::short_date,
    theme,
};

/// How long typing must pause before the query is sent.
pub const DEBOUNCE: Duration = Duration::from_millis(250);

const HEADER_SIZE: f32 = 13.0;
const EMPTY_PADDING: f32 = 32.0;
const NAME_PORTION: u16 = 3;
const LOCATION_PORTION: u16 = 2;

#[derive(Debug, Clone)]
pub enum Message {
    Changed(String),
    Debounced(u64),
    Loaded(u64, Result<Vec<SearchHit>, ApiError>),
    Open(usize),
    Clear,
}

/// What the browser has to do after a search update.
pub enum Outcome {
    Task(Task<Message>),
    /// Show the folder with this id (`None` for Home).
    OpenFolder(Option<NodeId>),
}

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

    pub fn update(&mut self, client: &Client, message: Message) -> Outcome {
        match message {
            Message::Changed(query) => match self.change(query) {
                Some(seq) => Outcome::Task(Task::perform(
                    async move {
                        tokio::time::sleep(DEBOUNCE).await;
                        seq
                    },
                    Message::Debounced,
                )),
                None => Outcome::Task(Task::none()),
            },
            Message::Debounced(seq) if self.accepts(seq) => {
                self.start();
                let client = client.clone();
                let query = self.query.trim().to_owned();
                Outcome::Task(Task::perform(
                    async move { client.search(&query).await },
                    move |result| Message::Loaded(seq, result),
                ))
            }
            Message::Debounced(_) => Outcome::Task(Task::none()),
            Message::Loaded(seq, result) => {
                if self.accepts(seq) {
                    self.finish(result);
                }
                Outcome::Task(Task::none())
            }
            Message::Open(index) => match self.hit(index).map(destination) {
                Some(folder) => Outcome::OpenFolder(folder),
                None => Outcome::Task(Task::none()),
            },
            Message::Clear => {
                self.clear();
                Outcome::Task(Task::none())
            }
        }
    }

    /// Column titles and the scrolling result rows, or a hint when there is nothing to list.
    pub fn view(&self) -> Element<'_, BrowserMessage> {
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

fn header<'a>() -> Element<'a, BrowserMessage> {
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

fn hit_row(index: usize, hit: &SearchHit) -> Element<'_, BrowserMessage> {
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
            ellipsized(node.name.as_str(), ROW_SIZE, Alignment::Start).width(Length::Fill)
        ]
        .spacing(CELL_SPACING / 1.5)
        .align_y(Alignment::Center)
        .width(Length::FillPortion(NAME_PORTION)),
        ellipsized(location_label(&hit.location), ROW_SIZE, Alignment::Start)
            .width(Length::FillPortion(LOCATION_PORTION)),
        ellipsized(size, ROW_SIZE, Alignment::End).width(SIZE_WIDTH),
        ellipsized(short_date(node.updated_at), ROW_SIZE, Alignment::Start).width(DATE_WIDTH),
    ]
    .spacing(CELL_SPACING)
    .align_y(Alignment::Center);
    button(cells)
        .width(Length::Fill)
        .padding(ROW_PADDING)
        .style(theme::row(false))
        .on_press(BrowserMessage::Search(Message::Open(index)))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

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
