use yew::prelude::*;

use home_space_contracts::files::FileNode;

use crate::user::secure_component::use_user_context;
use super::file_api::use_nodes;
use super::file_list_header_component::FileListHeader;


#[derive(Properties, PartialEq)]
pub struct FileListProps {
    pub parent_id: i64,
    pub open_folder: Callback<i64>
}

#[function_component(FileList)]
pub fn file_nodes_component(props: &FileListProps) -> HtmlResult {
    let user = use_user_context();
    let nodes = use_nodes(props.parent_id, &user.access_token.token)?;

    let on_folder_click = |id: i64| {
        let open_folder = props.open_folder.clone();
        Callback::from(move |_| {
            open_folder.emit(id)
        })
    };

    Ok(html! {
        <div class="file-list">
            <FileListHeader />
            {
                nodes.iter().map(|node: &FileNode| {
                    html!{
                        <div key={node.id} class="file-list-row">
                            <div></div>
                            <div class="file-list-title"  onclick={on_folder_click(node.id)}>
                                <span class="icon-filled">{get_node_icon(&node)}</span>
                                <span>{node.title.clone()}</span>
                            </div>
                            <div>{node.node_size.clone()}</div>
                            <div>{node.modified_at.clone()}</div>
                        </div>
                    }
                }).collect::<Html>()
            }
        </div>
    })
}

fn get_node_icon(node: &FileNode) -> &str {
    if node.node_type == 0 {
        return "folder";
    }
    match node.mime_type {
        _ => "insert_drive_file"
    }
}