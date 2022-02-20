use yew::prelude::*;

use super::file_list_component::FileList;

#[function_component(FilesView)]
pub fn file_view() -> Html {
    let fallback = html! {<>{"Loading..."}</> };
    html!{
        <Suspense fallback={fallback}>
            <FileList parent_id={0} />
        </Suspense>
    }
}
