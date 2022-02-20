use yew::prelude::*;

use home_space_contracts::files::FileNode;

use crate::user::secure_component::use_user_context;
use super::file_api::use_nodes;

#[derive(Properties, PartialEq)]
pub struct FileListProps {
    pub parent_id: i64
}

#[function_component(FileList)]
pub fn file_nodes_component(props: &FileListProps) -> HtmlResult {
    let user = use_user_context();
    let nodes = use_nodes(props.parent_id, &user.access_token.token)?;
    Ok(html! {
        <div class="file-list">
        {
            nodes.iter().map(|node: &FileNode| {
                html!(<div key={node.id}>{node.title.clone()}</div>)
            }).collect::<Html>()
        }
        </div>
    })
}
