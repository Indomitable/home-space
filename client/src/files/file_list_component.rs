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
        <>
        {
            nodes.iter().map(|node: &FileNode| {
                html!{
                    <div key={node.id}>
                        <div>{node.title.clone()}</div>
                        <div>{node.node_size.clone()}</div>
                        <div>{node.modified_at.clone()}</div>
                    </div>
                }
            }).collect::<Html>()
        }
        </>
    })
}
