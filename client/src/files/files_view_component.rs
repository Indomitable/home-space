use yew::prelude::*;

use super::file_list_component::FileList;
use super::actions::file_actions_component::FileActions;
use super::breadcrumbs::breadcrumbs_component::BreadcumbsFileNav;

#[function_component(FilesView)]
pub fn file_view() -> Html {
    let fallback = html! {<>{"Loading..."}</> };
    let parent_id_state = use_state(|| 0);

    let parent_id = *parent_id_state;
    let on_folder_click: Callback<i64> = Callback::from(move |id: i64| {
        parent_id_state.set(id);
    });


    html!{
        <>
            <FileActions parent_id={parent_id} />
            <BreadcumbsFileNav />
            <Suspense fallback={fallback}>
                <FileList parent_id={parent_id} open_folder={on_folder_click} />
            </Suspense>
        </>
    }
}
