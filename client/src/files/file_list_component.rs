use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;
use yew_router::prelude::*;

use home_space_contracts::files::FileNode;

use super::file_api::use_nodes;
use super::file_list_header_component::FileListHeader;


#[derive(Properties, PartialEq)]
pub struct FileListProps {
    pub parent_id: i64,
    pub access_token: String,
}

#[function_component(FileList)]
pub fn file_nodes_component(props: &FileListProps) -> HtmlResult {
    
    let nodes = use_nodes(props.parent_id, &props.access_token)?;

    Ok(html! {
        <div class="file-list">
            <FileListHeader />
            {
                nodes.iter().map(|node: &FileNode| {
                    html!{
                        <NodeRow node={node.clone()} />
                    }
                }).collect::<Html>()
            }
        </div>
    })
}



#[derive(Properties, PartialEq)]
struct NodeRowProps {
    node: FileNode
}

#[function_component(NodeRow)]
fn node_row(props: &NodeRowProps) -> Html {
    let FileNode { id, title, parent_id: _, node_type, mime_type, modified_at, node_size } = &props.node;
    let navigator = use_navigator().unwrap_throw();
    let onclick = {
        let id = *id;
        Callback::from(move |_| {
            navigator.push(crate::router::AppRoute::FileList{ parent_id: id });
        })
    };

    html!{
        <div key={*id} class="file-list-row">
            <div></div>
            <div class="file-list-title"  onclick={onclick}>
                <span class="icon-filled">{get_node_icon(*node_type, &mime_type)}</span>
                <span>{title.clone()}</span>
            </div>
            <div>{node_size.clone()}</div>
            <div>{modified_at.clone()}</div>
        </div>
    }
}

fn get_node_icon<'a>(node_type: i16, mime_type: &'a str) -> &'a str {
    if node_type == 0 {
        return "folder";
    }
    match mime_type {
        _ => "insert_drive_file"
    }
}
