use yew::prelude::*;
use yew_router::prelude::*;

use crate::router::AppRoute;

use super::file_list_component::FileList;
use super::actions::file_actions_component::FileActions;
use super::breadcrumbs::breadcrumbs_component::BreadcumbsFileNav;

#[derive(Properties, PartialEq)]
pub struct FilesViewProps {
    pub parent_id: i64
}


#[function_component(FilesView)]
pub fn file_view(props: &FilesViewProps) -> Html {
    let fallback = html! {<>{"Loading..."}</> };
    let navigator = use_navigator().expect("Should have navigator");
    let on_folder_click: Callback<i64> = Callback::from(move |id: i64| {
        navigator.push(AppRoute::FileList{parent_id: id});
    });

    html!{
        <>
            <FileActions parent_id={props.parent_id} />
            <BreadcumbsFileNav />
            <Suspense fallback={fallback}>
                <FileList parent_id={props.parent_id} open_folder={on_folder_click} />
            </Suspense>
        </>
    }
}
