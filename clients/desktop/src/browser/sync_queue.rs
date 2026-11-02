//! The sync queue: what the sync still has to do, with the progress of the transfers.

use iced::{
    Alignment, Element, Length, Padding,
    widget::{column, container, progress_bar, row, scrollable, text},
};

use client_core::format::human_size;

use super::{
    Message,
    grid::{
        CELL_SPACING, EMPTY_PADDING, HEADER_SIZE, ROW_GAP, ROW_PADDING, ROW_SIZE, SCROLLBAR_GUTTER,
    },
    syncing::display_path,
};
use crate::{
    ellipsis::ellipsized,
    icons::{ICON_SIZE, Icon, Tint},
    sync::{Activity, Item, ItemState, Operation, Status, Summary},
    theme,
};

/// Rows shown before the rest is only counted; a queue can hold thousands of files.
const MAX_ROWS: usize = 500;
const SPACING: f32 = 4.0;
const NOTE_SIZE: f32 = 13.0;
const PATH_SIZE: f32 = 12.0;
const PROGRESS_WIDTH: f32 = 220.0;
const PROGRESS_GIRTH: f32 = 6.0;
const STATE_WIDTH: f32 = 80.0;

/// A titled group of the queue and which states belong to it.
type Section = (&'static str, fn(&ItemState) -> bool);

pub fn view(status: &Status) -> Element<'_, Message> {
    let summary = status.summary();
    let busy = match status.activity {
        Activity::Checking => Some("Checking server…"),
        Activity::Scanning => Some("Scanning…"),
        Activity::Idle | Activity::Syncing => None,
    };
    let mut page = column![].spacing(SPACING * 3.0);
    let line = summary_line(summary);
    if let Some(line) = line.as_deref().or(busy) {
        page = page.push(text(line.to_owned()));
    }
    if status.queue.is_empty() {
        return page
            .push(
                container(text("Everything is up to date."))
                    .padding(EMPTY_PADDING)
                    .center_x(Length::Fill),
            )
            .into();
    }

    let mut shown = 0;
    let mut rows = column![]
        .spacing(ROW_GAP)
        .padding(Padding::default().right(SCROLLBAR_GUTTER));
    let sections: [Section; 3] = [
        ("Syncing now", |state| *state == ItemState::Active),
        ("Waiting", |state| *state == ItemState::Queued),
        ("Could not sync", |state| {
            matches!(state, ItemState::Failed(_))
        }),
    ];
    for (title, wanted) in sections {
        let items: Vec<&Item> = status
            .queue
            .iter()
            .filter(|item| wanted(&item.state))
            .collect();
        if items.is_empty() {
            continue;
        }
        rows = rows.push(
            text(format!("{title} ({})", items.len()))
                .size(HEADER_SIZE)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..iced::Font::DEFAULT
                }),
        );
        for item in items.into_iter().take(MAX_ROWS.saturating_sub(shown)) {
            shown += 1;
            rows = rows.push(item_row(item));
        }
    }
    let hidden = status.queue.len().saturating_sub(shown);
    if hidden > 0 {
        rows = rows.push(text(format!("and {hidden} more")).size(NOTE_SIZE));
    }
    page.push(scrollable(rows).height(Length::Fill)).into()
}

/// "Uploading 12 · Downloading 3 · 4.0 MB left", or `None` with nothing to do.
fn summary_line(summary: Summary) -> Option<String> {
    let counts = [
        ("Uploading", summary.uploads),
        ("Downloading", summary.downloads),
        ("Other changes", summary.others),
    ]
    .into_iter()
    .filter(|(_, count)| *count > 0)
    .map(|(label, count)| format!("{label} {count}"));
    let mut parts: Vec<String> = counts.collect();
    if summary.bytes_left > 0 {
        let left = i64::try_from(summary.bytes_left).unwrap_or(i64::MAX);
        parts.push(format!("{} left", human_size(left)));
    }
    if summary.failed > 0 {
        parts.push(format!("{} failed", summary.failed));
    }
    (!parts.is_empty()).then(|| parts.join(" · "))
}

fn item_row(item: &Item) -> Element<'_, Message> {
    let (icon, tint) = match (&item.state, item.operation) {
        (ItemState::Failed(_), _) => (Icon::SyncProblem, Tint::Danger),
        (_, Operation::Upload) => (Icon::Upload, Tint::Muted),
        (_, Operation::Download) => (Icon::Download, Tint::Muted),
        (_, Operation::CreateRemoteFolder | Operation::CreateLocalFolder) => {
            (Icon::NewFolder, Tint::Muted)
        }
        (_, Operation::Move) => (Icon::Move, Tint::Muted),
        (_, Operation::Delete) => (Icon::Delete, Tint::Muted),
        (_, Operation::Blocked) => (Icon::SyncProblem, Tint::Danger),
    };
    let note = match &item.state {
        ItemState::Failed(reason) => reason.clone(),
        _ => display_path(&item.path),
    };
    let name = column![
        ellipsized(item.name.as_str(), ROW_SIZE, Alignment::Start).width(Length::Fill),
        container(ellipsized(note, PATH_SIZE, Alignment::Start).width(Length::Fill))
            .style(theme::muted),
    ]
    .width(Length::Fill);
    let cells = row![
        icon.view(ICON_SIZE, tint),
        name,
        container(progress(item)).width(PROGRESS_WIDTH),
        ellipsized(state_label(&item.state), ROW_SIZE, Alignment::Start).width(STATE_WIDTH),
    ]
    .spacing(CELL_SPACING)
    .align_y(Alignment::Center);
    container(cells)
        .padding(ROW_PADDING)
        .width(Length::Fill)
        .into()
}

/// The bytes moved so far with a bar while active, otherwise the size.
fn progress(item: &Item) -> Element<'_, Message> {
    let size = |bytes: u64| human_size(i64::try_from(bytes).unwrap_or(i64::MAX));
    if item.size == 0 {
        return text("").into();
    }
    if item.state != ItemState::Active {
        return text(size(item.size)).size(ROW_SIZE).into();
    }
    column![
        progress_bar(0.0..=item.size as f32, item.done as f32)
            .length(Length::Fill)
            .girth(PROGRESS_GIRTH),
        text(format!("{} of {}", size(item.done), size(item.size))).size(PATH_SIZE),
    ]
    .spacing(SPACING)
    .into()
}

fn state_label(state: &ItemState) -> &'static str {
    match state {
        ItemState::Queued => "Waiting",
        ItemState::Active => "Syncing",
        ItemState::Failed(_) => "Failed",
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use iced_test::simulator;

    use super::*;
    use crate::sync::Operation;

    fn item(name: &str, operation: Operation, size: u64, done: u64, state: ItemState) -> Item {
        Item::test(
            PathBuf::from("/sync").join(name),
            operation,
            size,
            done,
            state,
        )
    }

    fn status(queue: Vec<Item>) -> Status {
        Status {
            queue,
            ..Status::default()
        }
    }

    #[test]
    fn an_empty_queue_says_everything_is_up_to_date() {
        let status = status(Vec::new());
        let mut ui = simulator(view(&status));
        assert!(ui.find("Everything is up to date.").is_ok());
    }

    #[test]
    fn the_queue_is_grouped_by_state_with_progress_on_the_active_ones() {
        let status = status(vec![
            item(
                "big.mov",
                Operation::Upload,
                80 << 20,
                12 << 20,
                ItemState::Active,
            ),
            item("a.txt", Operation::Download, 2048, 0, ItemState::Queued),
            item(
                "bad.txt",
                Operation::Upload,
                1,
                0,
                ItemState::Failed("boom".to_owned()),
            ),
        ]);
        let mut ui = simulator(view(&status));
        assert!(ui.find("Syncing now (1)").is_ok());
        assert!(ui.find("Waiting (1)").is_ok());
        assert!(ui.find("Could not sync (1)").is_ok());
        assert!(ui.find("12.0 MB of 80.0 MB").is_ok());
        assert!(ui.find("boom").is_ok());
    }

    #[test]
    fn a_long_queue_is_cut_off_and_counted() {
        let queue = (0..MAX_ROWS + 20)
            .map(|n| {
                item(
                    &format!("f{n}.txt"),
                    Operation::Upload,
                    1,
                    0,
                    ItemState::Queued,
                )
            })
            .collect();
        let status = status(queue);
        let mut ui = simulator(view(&status));
        assert!(ui.find("and 20 more").is_ok());
    }

    #[test]
    fn the_summary_names_what_is_ahead() {
        let summary = Summary {
            uploads: 12,
            downloads: 3,
            failed: 1,
            bytes_left: 4096,
            ..Summary::default()
        };
        assert_eq!(
            summary_line(summary).as_deref(),
            Some("Uploading 12 · Downloading 3 · 4.0 KB left · 1 failed")
        );
        assert_eq!(summary_line(Summary::default()), None);
    }
}
