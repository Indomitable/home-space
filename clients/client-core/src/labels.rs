use protocol::Node;
use time::OffsetDateTime;

use crate::format::human_size;

/// The name of the root folder.
pub const HOME: &str = "Home";

/// The name of the folder that receives restored files whose folder is gone.
pub const LOST_AND_FOUND: &str = "Lost & Found";

const LOCATION_SEPARATOR: &str = " / ";

/// Where a search hit lives, e.g. `Home / Photos / 2026`.
pub fn location_label(location: &[String]) -> String {
    std::iter::once(HOME)
        .chain(location.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(LOCATION_SEPARATOR)
}

/// `1 file`, `2 files`.
pub fn files(count: usize) -> String {
    match count {
        1 => "1 file".to_owned(),
        _ => format!("{count} files"),
    }
}

/// A node's modification time and size, e.g. `2026-03-04 05:06 · 1.5 KB`; `short_date`
/// formats the time, since each client follows its platform's regional settings.
pub fn describe(node: &Node, short_date: impl Fn(OffsetDateTime) -> String) -> String {
    format!(
        "{} · {}",
        short_date(node.updated_at),
        human_size(node.size)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locations_start_at_home() {
        assert_eq!(location_label(&[]), "Home");
        assert_eq!(
            location_label(&["a".to_owned(), "b".to_owned()]),
            "Home / a / b"
        );
    }

    #[test]
    fn file_counts_agree_with_the_number() {
        assert_eq!(files(1), "1 file");
        assert_eq!(files(0), "0 files");
        assert_eq!(files(3), "3 files");
    }
}
