use leptos::prelude::*;
use web_sys::{File, HtmlInputElement};

use super::{
    files_of,
    menu::{Menu, MenuItem},
};

#[component]
pub fn Toolbar(
    selected_count: Signal<usize>,
    on_move: Callback<()>,
    on_delete: Callback<()>,
    can_delete: Signal<bool>,
    can_add: Signal<bool>,
    on_new_folder: Callback<()>,
    on_upload: Callback<Vec<File>>,
) -> impl IntoView {
    view! {
        <div class="explorer__toolbar">
            <Show when=move || selected_count.get() != 0>
                <Menu
                    id="bulk-menu".to_owned()
                    label="Actions for selected items"
                    text=Signal::derive(move || format!("{} selected", selected_count.get()))
                    icon="more_horiz"
                >
                    <MenuItem icon="drive_file_move" label="Move" on_select=on_move />
                    <Show when=move || can_delete.get()>
                        <MenuItem icon="delete" label="Delete" danger=true on_select=on_delete />
                    </Show>
                </Menu>
            </Show>
            <button
                class="button"
                disabled=move || !can_add.get()
                on:click=move |_| on_new_folder.run(())
            >
                "New folder"
            </button>
            <label class="button button--primary" class:button--disabled=move || !can_add.get()>
                "Upload files"
                <input
                    type="file"
                    multiple
                    hidden
                    disabled=move || !can_add.get()
                    on:change:target=move |event| {
                        let input: HtmlInputElement = event.target();
                        if let Some(list) = input.files() {
                            on_upload.run(files_of(&list));
                        }
                        input.set_value("");
                    }
                />
            </label>
        </div>
    }
}
