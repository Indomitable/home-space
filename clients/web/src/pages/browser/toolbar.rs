use leptos::prelude::*;
use web_sys::{File, HtmlInputElement};

use super::files_of;

#[component]
pub fn Toolbar(on_new_folder: Callback<()>, on_upload: Callback<Vec<File>>) -> impl IntoView {
    view! {
        <div class="explorer__toolbar">
            <button class="button" on:click=move |_| on_new_folder.run(())>
                "New folder"
            </button>
            <label class="button button--primary">
                "Upload files"
                <input
                    type="file"
                    multiple
                    hidden
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
