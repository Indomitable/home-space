use protocol::{Node, SearchHit};
use time::OffsetDateTime;
use uuid::Uuid;

use iced_test::simulator;

use super::*;

const FOLDER_NAME: &str = "Folder B";
const FILE_NAME: &str = "report.pdf";
const OTHER_FILE_NAME: &str = "notes.txt";
const FOLDER_ID: u128 = 1;
const FILE_ID: u128 = 2;
const OTHER_FILE_ID: u128 = 3;

fn id(raw: u128) -> NodeId {
    NodeId(Uuid::from_u128(raw))
}

fn node(name: &str, kind: NodeKind, raw: u128) -> Node {
    Node {
        id: id(raw),
        parent_id: None,
        kind,
        name: name.to_owned(),
        size: 2048,
        sha256: None,
        mime: String::new(),
        version: 1,
        created_at: OffsetDateTime::UNIX_EPOCH,
        updated_at: OffsetDateTime::UNIX_EPOCH,
    }
}

fn state_with_listing() -> State {
    let me = Me {
        id: protocol::UserId(Uuid::nil()),
        username: "alice".to_owned(),
        lost_and_found_id: NodeId(Uuid::nil()),
    };
    let (mut state, _task) = State::open(Client::new("http://localhost"), me);
    let listing = Listing {
        path: Vec::new(),
        nodes: vec![
            node(FOLDER_NAME, NodeKind::Folder, FOLDER_ID),
            node(FILE_NAME, NodeKind::File, FILE_ID),
            node(OTHER_FILE_NAME, NodeKind::File, OTHER_FILE_ID),
        ],
    };
    let _ = state.update(Message::Loaded(None, Ok(listing)));
    state
}

#[test]
fn clicking_a_row_selects_it() {
    let state = state_with_listing();
    let mut ui = simulator(state.view());

    ui.click(FILE_NAME).expect("file row is shown");

    let messages: Vec<_> = ui.into_messages().collect();
    assert!(matches!(messages.as_slice(), [Message::Click(clicked)] if *clicked == id(FILE_ID)));
}

#[test]
fn a_click_selects_only_that_row() {
    let mut state = state_with_listing();

    let _ = state.update(Message::Click(id(FILE_ID)));

    assert_eq!(state.folder.selected().len(), 1);
    assert!(state.folder.selection.contains(id(FILE_ID)));
}

#[test]
fn the_open_details_panel_follows_the_selected_row() {
    let mut state = state_with_listing();
    let _ = state.update(Message::Click(id(FILE_ID)));
    let _ = state.update(Message::Act(Action::Details));
    assert_eq!(
        state.details.as_ref().map(details::State::node_id),
        Some(id(FILE_ID))
    );

    let _ = state.update(Message::Click(id(OTHER_FILE_ID)));

    assert_eq!(
        state.details.as_ref().map(details::State::node_id),
        Some(id(OTHER_FILE_ID))
    );
}

#[test]
fn selecting_a_row_does_not_open_a_closed_details_panel() {
    let mut state = state_with_listing();

    let _ = state.update(Message::Click(id(FILE_ID)));

    assert!(state.details.is_none());
}

#[test]
fn double_clicking_a_folder_opens_it() {
    let mut state = state_with_listing();

    let _ = state.update(Message::Click(id(FOLDER_ID)));
    let _ = state.update(Message::Click(id(FOLDER_ID)));

    assert_eq!(state.folder.id, Some(id(FOLDER_ID)));
    assert!(state.folder.loading);
}

#[test]
fn rename_is_disabled_for_two_selected_items() {
    let mut state = state_with_listing();
    let _ = state.update(Message::Toggle(id(FILE_ID)));
    let _ = state.update(Message::Toggle(id(OTHER_FILE_ID)));
    let mut ui = simulator(state.view());

    ui.click("Rename").expect("rename is shown");

    assert_eq!(ui.into_messages().count(), 0);
}

fn inside_lost_and_found() -> State {
    let mut state = state_with_listing();
    state.me.lost_and_found_id = id(FOLDER_ID);
    state.folder.id = Some(id(FOLDER_ID));
    state
}

#[test]
fn nothing_can_be_created_inside_lost_and_found() {
    let mut state = inside_lost_and_found();

    let _ = state.update(Message::NewFolder);
    let _ = state.update(Message::Shortcut(Shortcut::NewFolder));
    let _ = state.update(Message::Upload);
    let _ = state.update(Message::FileHovered);

    assert!(state.dialog.is_none());
    assert!(!state.dropping);
}

#[test]
fn folders_and_uploads_still_work_elsewhere() {
    let mut creating = state_with_listing();
    let _ = creating.update(Message::NewFolder);
    assert!(creating.dialog.is_some());

    let mut hovered = state_with_listing();
    let _ = hovered.update(Message::FileHovered);
    assert!(hovered.dropping);
}

#[test]
fn delete_does_nothing_for_lost_and_found() {
    let mut state = state_with_listing();
    state.me.lost_and_found_id = id(FOLDER_ID);
    let _ = state.update(Message::Toggle(id(FOLDER_ID)));
    let _ = state.update(Message::Toggle(id(FILE_ID)));

    let _ = state.update(Message::Shortcut(Shortcut::Delete));
    let _ = state.update(Message::Act(Action::Delete));

    assert!(state.dialog.is_none());
}

#[test]
fn context_menu_delete_opens_the_delete_dialog() {
    let mut state = state_with_listing();

    let _ = state.update(Message::ContextMenu(id(FILE_ID)));
    let _ = state.update(Message::Act(Action::Delete));

    assert!(state.menu.is_none());
    assert!(state.dialog.is_some());
}

#[test]
fn escape_closes_the_dialog() {
    let mut state = state_with_listing();
    let _ = state.update(Message::NewFolder);

    let _ = state.update(Message::Shortcut(Shortcut::Escape));

    assert!(state.dialog.is_none());
}

#[test]
fn creating_a_folder_with_an_empty_name_closes_the_dialog() {
    let mut state = state_with_listing();
    let _ = state.update(Message::NewFolder);

    let _ = state.update(Message::Dialog(dialogs::Message::Submit));

    assert!(state.dialog.is_none());
}

#[test]
fn backspace_goes_to_the_parent_folder() {
    let mut state = state_with_listing();
    let _ = state.update(Message::Jump(Some(id(FOLDER_ID))));
    let listing = Listing {
        path: vec![node(FOLDER_NAME, NodeKind::Folder, FOLDER_ID)],
        nodes: Vec::new(),
    };
    let _ = state.update(Message::Loaded(Some(id(FOLDER_ID)), Ok(listing)));

    let _ = state.update(Message::Shortcut(Shortcut::Back));

    assert_eq!(state.folder.id, None);
}

#[test]
fn back_after_a_breadcrumb_jump_returns_to_the_folder_it_left() {
    let mut state = state_with_listing();
    open_deep_path(&mut state, 3);
    let deep = state.folder.id;

    let _ = state.update(Message::Jump(None));
    assert_eq!(state.folder.id, None);
    let _ = state.update(Message::Back);

    assert_eq!(state.folder.id, deep);
    // The jump is spent: the next back goes to the parent.
    let _ = state.update(Message::Loaded(
        deep,
        Ok(Listing {
            path: (1..=3)
                .map(|raw| node(&format!("level-{raw}"), NodeKind::Folder, 100 + raw))
                .collect(),
            nodes: Vec::new(),
        }),
    ));
    let _ = state.update(Message::Back);
    assert_eq!(state.folder.id, Some(id(102)));
}

fn open_deep_path(state: &mut State, depth: u128) {
    let path: Vec<Node> = (1..=depth)
        .map(|raw| node(&format!("level-{raw}"), NodeKind::Folder, 100 + raw))
        .collect();
    let current = path.last().map(|folder| folder.id);
    let _ = state.update(Message::Jump(current));
    let listing = Listing {
        path,
        nodes: Vec::new(),
    };
    let _ = state.update(Message::Loaded(current, Ok(listing)));
}

#[test]
fn deep_breadcrumbs_hide_the_middle_folders() {
    let mut state = state_with_listing();
    open_deep_path(&mut state, 5);
    let mut ui = simulator(state.view());

    assert!(ui.find("level-1").is_ok());
    assert!(ui.find("level-5").is_ok());
    assert!(ui.find("level-3").is_err());
    assert_eq!(state.folder.hidden_crumbs().len(), 3);
}

#[test]
fn the_overflow_crumb_opens_the_hidden_folders_menu() {
    let mut state = state_with_listing();
    open_deep_path(&mut state, 4);

    let _ = state.update(Message::CrumbsMenu);
    let mut ui = simulator(state.view());

    assert!(matches!(state.menu, Some(Menu::Crumbs(_))));
    assert!(ui.find("level-2").is_ok());
}

#[test]
fn a_created_node_in_the_open_folder_appears() {
    let mut state = state_with_listing();
    let created = node("added.txt", NodeKind::File, 9);

    let update = state.update(Message::Server(protocol::Event::NodeCreated {
        node: created,
    }));

    assert!(matches!(update, Update::Task(task) if task.units() == 0));
    assert!(
        state
            .folder
            .listing
            .nodes
            .iter()
            .any(|node| node.id == id(9))
    );
}

#[test]
fn a_deleted_selected_node_leaves_the_selection() {
    let mut state = state_with_listing();
    let _ = state.update(Message::Click(id(FILE_ID)));

    let _ = state.update(Message::Server(protocol::Event::NodeDeleted {
        id: id(FILE_ID),
        parent_id: None,
    }));

    assert!(state.folder.selected().is_empty());
}

fn hit_for(node: Node) -> SearchHit {
    SearchHit {
        node,
        location: Vec::new(),
    }
}

fn searching(state: &mut State, query: &str) -> u64 {
    let _ = state.update(Message::Search(search::Message::Changed(query.to_owned())));
    state.search.seq
}

#[test]
fn typing_a_query_schedules_a_debounced_search() {
    let mut state = state_with_listing();

    let update = state.update(Message::Search(search::Message::Changed("rep".to_owned())));

    assert!(matches!(update, Update::Task(task) if task.units() > 0));
    assert!(state.search.is_active());
}

#[test]
fn a_blank_query_schedules_nothing() {
    let mut state = state_with_listing();

    let update = state.update(Message::Search(search::Message::Changed("  ".to_owned())));

    assert!(matches!(update, Update::Task(task) if task.units() == 0));
}

#[test]
fn results_of_an_outdated_query_are_ignored() {
    let mut state = state_with_listing();
    let stale = searching(&mut state, "a");
    searching(&mut state, "ab");

    let _ = state.update(Message::Search(search::Message::Loaded(
        stale,
        Ok(vec![hit_for(node(FILE_NAME, NodeKind::File, FILE_ID))]),
    )));

    assert!(state.search.hit(0).is_none());
}

#[test]
fn results_of_the_current_query_are_shown() {
    let mut state = state_with_listing();
    let seq = searching(&mut state, "rep");

    let _ = state.update(Message::Search(search::Message::Loaded(
        seq,
        Ok(vec![hit_for(node(FILE_NAME, NodeKind::File, FILE_ID))]),
    )));
    let mut ui = simulator(state.view());

    assert!(ui.find(FILE_NAME).is_ok());
}

#[test]
fn opening_a_folder_hit_leaves_the_search() {
    let mut state = state_with_listing();
    let seq = searching(&mut state, "folder");
    let _ = state.update(Message::Search(search::Message::Loaded(
        seq,
        Ok(vec![hit_for(node(
            FOLDER_NAME,
            NodeKind::Folder,
            FOLDER_ID,
        ))]),
    )));

    let _ = state.update(Message::Search(search::Message::Open(0)));

    assert_eq!(state.folder.id, Some(id(FOLDER_ID)));
    assert!(!state.search.is_active());
}

#[test]
fn escape_clears_an_active_search() {
    let mut state = state_with_listing();
    searching(&mut state, "rep");

    let _ = state.update(Message::Shortcut(Shortcut::Escape));

    assert!(!state.search.is_active());
}

#[test]
fn folder_shortcuts_do_nothing_while_searching() {
    let mut state = state_with_listing();
    let _ = state.update(Message::Click(id(FILE_ID)));
    searching(&mut state, "rep");

    let _ = state.update(Message::Shortcut(Shortcut::Delete));

    assert!(state.dialog.is_none());
}

#[test]
fn a_resync_reloads_the_listing() {
    let mut state = state_with_listing();

    let update = state.update(Message::Server(protocol::Event::Resync));

    assert!(matches!(update, Update::Task(task) if task.units() > 0));
}

#[test]
fn the_toolbar_opens_the_recycle_bin() {
    let state = state_with_listing();
    let mut ui = simulator(state.view());

    ui.click("Recycle bin").expect("the entry is shown");

    let messages: Vec<_> = ui.into_messages().collect();
    assert!(matches!(messages.as_slice(), [Message::OpenTrash]));
}

#[test]
fn opening_the_recycle_bin_loads_it() {
    let mut state = state_with_listing();

    let update = state.update(Message::OpenTrash);

    assert!(matches!(update, Update::Task(task) if task.units() > 0));
    assert!(state.trash.is_some());
}

#[test]
fn a_trash_change_reloads_the_open_bin_only() {
    let mut state = state_with_listing();
    let update = state.update(Message::Server(protocol::Event::TrashChanged));
    assert!(matches!(update, Update::Task(task) if task.units() == 0));

    let _ = state.update(Message::OpenTrash);
    let update = state.update(Message::Server(protocol::Event::TrashChanged));
    assert!(matches!(update, Update::Task(task) if task.units() > 0));
}

#[test]
fn opening_a_folder_leaves_the_recycle_bin() {
    let mut state = state_with_listing();
    let _ = state.update(Message::OpenTrash);

    let _ = state.update(Message::Jump(None));

    assert!(state.trash.is_none());
}

#[test]
fn restore_clashes_open_the_conflicts_dialog() {
    let mut state = state_with_listing();
    let _ = state.update(Message::OpenTrash);
    let response = protocol::MoveResponse {
        conflicts: vec![protocol::Conflict {
            source: node(FILE_NAME, NodeKind::File, FILE_ID),
            path: format!("/{FILE_NAME}"),
            existing: None,
        }],
        ..protocol::MoveResponse::default()
    };

    let _ = state.update(Message::Trash(trash::Message::Restored(Ok(response))));

    assert!(state.dialog.is_some());
}
