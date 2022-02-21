use yew::prelude::*;

use super::file_list_component::FileList;
use super::actions::file_actions_component::FileActions;
use super::breadcrumbs::breadcrumbs_component::BreadcumbsFileNav;

#[function_component(FilesView)]
pub fn file_view() -> Html {
    let fallback = html! {<>{"Loading..."}</> };
    html!{
        <>
            <FileActions />
            <BreadcumbsFileNav />
            <Suspense fallback={fallback}>
                <FileList parent_id={0} />
            </Suspense>
        </>
    }
}
