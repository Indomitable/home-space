use yew::prelude::*;

use crate::user::secure_component::use_user_context;
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

    let user = use_user_context();
    let token = user.access_token.token;
    html!{
        <>
            <FileActions parent_id={props.parent_id} />
            <BreadcumbsFileNav parent_id={props.parent_id} access_token={token.clone()} />
            <Suspense fallback={fallback}>
                <FileList parent_id={props.parent_id} access_token={token.clone()} />
            </Suspense>
        </>
    }
}
