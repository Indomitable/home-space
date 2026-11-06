//! Decides what a pass does with the files a source lists. Pure, so the rules can be read in the
//! tests below.
//!
//! Only size and modification time are compared with the record of what was last sent, and
//! nothing on the server is looked at. So a file moved or deleted on the server is left alone until
//! it changes here, and then goes up by name to where it was first put, with the id it got: the
//! server makes that a new version if that file still has the name, keeps both if a different file
//! took it, and makes a new file if it is free. A file gone from the source changes nothing, and a
//! renamed one is a new file.

use std::collections::HashMap;

use protocol::NodeId;

/// A file a source lists; `mtime` is in nanoseconds since the Unix epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Item {
    pub key: String,
    pub dir: String,
    pub name: String,
    pub size: u64,
    pub mtime: i64,
    pub mime: Option<String>,
}

/// What is known about a file of a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Record {
    pub key: String,
    pub size: u64,
    pub mtime: i64,
    /// Where it was sent: the name the server gave it, in the folder at `remote_dir` below the
    /// target. `None` for a file that was already there when the source was added and was left out.
    pub remote_name: Option<String>,
    pub remote_dir: String,
    /// The file on the server, whose new version an edit is. `None` if left out, or sent before
    /// the id was kept: then an edit never becomes a version of what has the name.
    pub remote_node: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Step {
    /// Has not been sent before. A different file with its name on the server is kept too.
    First(Item),
    /// Changed since it was sent. It goes to the same name, as a new version of `node` if that
    /// is still the file there.
    Again {
        item: Item,
        name: String,
        dir: String,
        node: Option<NodeId>,
    },
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Plan {
    pub steps: Vec<Step>,
    /// Were already there when the source was added, and the user chose not to upload those.
    pub left_out: Vec<Item>,
    /// Files changed so recently that something may still be writing them.
    pub settling: usize,
}

/// How recently a file must have changed for a pass to leave it for later, in nanoseconds.
pub(super) const SETTLE_NANOS: i64 = 5_000_000_000;

/// The steps for `items`. `leave_out_existing` is set on the first pass of a source that doesn't
/// upload what it already holds.
pub(super) fn plan(
    items: &[Item],
    records: &HashMap<String, Record>,
    leave_out_existing: bool,
    now: i64,
) -> Plan {
    let mut plan = Plan::default();
    for item in items {
        let record = records.get(&item.key);
        if record.is_some_and(|record| record.size == item.size && record.mtime == item.mtime) {
            continue;
        }
        if leave_out_existing && record.is_none() {
            plan.left_out.push(item.clone());
            continue;
        }
        if now.saturating_sub(item.mtime) < SETTLE_NANOS {
            plan.settling += 1;
            continue;
        }
        plan.steps.push(match record {
            Some(Record {
                remote_name: Some(name),
                remote_dir,
                remote_node,
                ..
            }) => Step::Again {
                item: item.clone(),
                name: name.clone(),
                dir: remote_dir.clone(),
                node: *remote_node,
            },
            _ => Step::First(item.clone()),
        });
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000_000_000_000;
    const OLD: i64 = NOW - 60_000_000_000;
    const NODE: &str = "0193a4a0-0000-7000-8000-000000000042";

    fn item(key: &str, size: u64, mtime: i64) -> Item {
        Item {
            key: key.to_owned(),
            dir: "2026".to_owned(),
            name: format!("{key}.jpg"),
            size,
            mtime,
            mime: None,
        }
    }

    fn records(records: &[Record]) -> HashMap<String, Record> {
        records
            .iter()
            .map(|record| (record.key.clone(), record.clone()))
            .collect()
    }

    fn sent(key: &str, size: u64, mtime: i64, name: &str) -> Record {
        Record {
            key: key.to_owned(),
            size,
            mtime,
            remote_name: Some(name.to_owned()),
            remote_dir: "2026".to_owned(),
            remote_node: Some(NODE.parse().expect("id")),
        }
    }

    #[test]
    fn a_new_photo_goes_up_keeping_any_file_with_its_name() {
        let new = item("a", 10, OLD);
        let plan = plan(std::slice::from_ref(&new), &HashMap::new(), false, NOW);
        assert_eq!(plan.steps, vec![Step::First(new)]);
    }

    #[test]
    fn a_file_sent_and_unchanged_stays_put_whatever_happened_on_the_server() {
        // Nothing on the server is an input: moving the photo there can't cause an upload.
        let known = records(&[sent("a", 10, OLD, "a.jpg")]);
        let plan = plan(&[item("a", 10, OLD)], &known, false, NOW);
        assert_eq!(plan, Plan::default());
    }

    #[test]
    fn an_edited_file_goes_to_the_name_it_was_sent_under() {
        // `a (1).jpg`: the server renamed it because of a clash. The edit must follow that name, or
        // it would become a version of the file it was kept apart from.
        let known = records(&[sent("a", 10, OLD, "a (1).jpg")]);
        let edited = item("a", 11, OLD + 1);
        let plan = plan(std::slice::from_ref(&edited), &known, false, NOW);
        assert_eq!(
            plan.steps,
            vec![Step::Again {
                item: edited,
                name: "a (1).jpg".to_owned(),
                dir: "2026".to_owned(),
                node: Some(NODE.parse().expect("id")),
            }]
        );
    }

    #[test]
    fn a_file_with_only_a_new_time_counts_as_edited() {
        let known = records(&[sent("a", 10, OLD, "a.jpg")]);
        let plan = plan(&[item("a", 10, OLD + 1)], &known, false, NOW);
        assert!(matches!(plan.steps[..], [Step::Again { .. }]));
    }

    #[test]
    fn a_file_gone_from_the_source_changes_nothing() {
        let known = records(&[sent("a", 10, OLD, "a.jpg")]);
        assert_eq!(plan(&[], &known, false, NOW), Plan::default());
    }

    #[test]
    fn a_renamed_file_is_a_new_file() {
        let known = records(&[sent("a", 10, OLD, "a.jpg")]);
        let renamed = item("b", 10, OLD);
        let plan = plan(std::slice::from_ref(&renamed), &known, false, NOW);
        assert_eq!(plan.steps, vec![Step::First(renamed)]);
    }

    #[test]
    fn existing_files_can_be_left_out_on_the_first_pass() {
        let existing = item("a", 10, OLD);
        let plan = plan(std::slice::from_ref(&existing), &HashMap::new(), true, NOW);
        assert_eq!(plan.left_out, vec![existing]);
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn a_file_left_out_and_edited_later_goes_up_as_new() {
        let known = records(&[Record {
            remote_name: None,
            remote_node: None,
            ..sent("a", 10, OLD, "")
        }]);
        let unchanged = plan(&[item("a", 10, OLD)], &known, false, NOW);
        assert_eq!(unchanged, Plan::default());
        let edited = item("a", 11, OLD + 1);
        let changed = plan(std::slice::from_ref(&edited), &known, false, NOW);
        assert_eq!(changed.steps, vec![Step::First(edited)]);
    }

    #[test]
    fn a_file_still_being_written_waits() {
        let plan = plan(&[item("a", 10, NOW - 1)], &HashMap::new(), false, NOW);
        assert_eq!(plan.steps, Vec::new());
        assert_eq!(plan.settling, 1);
    }
}
