use yew::prelude::*;

use super::file_list_component::FileList;
use super::file_actions_component::FileActions;

#[function_component(FilesView)]
pub fn file_view() -> Html {
    let fallback = html! {<>{"Loading..."}</> };
    html!{
        <>
            <FileActions />
            <Suspense fallback={fallback}>
                <FileList parent_id={0} />
            </Suspense>
        </>
    }
}
