use leptos::{ev, prelude::*};
use web_sys::{DragEvent, File, FileList};

const FILES_TYPE: &str = "Files";
const DROP_EFFECT_COPY: &str = "copy";
const DROP_EFFECT_NONE: &str = "none";

fn carries_files(event: &DragEvent) -> bool {
    event
        .data_transfer()
        .is_some_and(|transfer| transfer.types().includes(&FILES_TYPE.into(), 0))
}

pub(super) fn files_of(list: &FileList) -> Vec<File> {
    (0..list.length())
        .filter_map(|index| list.get(index))
        .collect()
}

/// Accepts files dropped anywhere on the window and passes them to `on_drop`; the result
/// is true while files are dragged over the window and `can_add` allows them.
pub(super) fn use_file_drop(can_add: Signal<bool>, on_drop: Callback<Vec<File>>) -> Memo<bool> {
    let drag_depth = RwSignal::new(0_u32);
    let on_enter = window_event_listener(ev::dragenter, move |event| {
        if carries_files(&event) {
            event.prevent_default();
            if can_add.get_untracked() {
                drag_depth.update(|depth| *depth += 1);
            }
        }
    });
    let on_over = window_event_listener(ev::dragover, move |event| {
        if carries_files(&event) {
            event.prevent_default();
            if let Some(transfer) = event.data_transfer() {
                let effect = if can_add.get_untracked() {
                    DROP_EFFECT_COPY
                } else {
                    DROP_EFFECT_NONE
                };
                transfer.set_drop_effect(effect);
            }
        }
    });
    let on_leave = window_event_listener(ev::dragleave, move |event| {
        if carries_files(&event) {
            drag_depth.update(|depth| *depth = depth.saturating_sub(1));
        }
    });
    let on_drop_event = window_event_listener(ev::drop, move |event| {
        event.prevent_default();
        drag_depth.set(0);
        if let Some(list) = event.data_transfer().and_then(|transfer| transfer.files()) {
            on_drop.run(files_of(&list));
        }
    });
    on_cleanup(move || {
        on_enter.remove();
        on_over.remove();
        on_leave.remove();
        on_drop_event.remove();
    });
    Memo::new(move |_| drag_depth.get() > 0)
}
