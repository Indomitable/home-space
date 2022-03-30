use yew::prelude::*;

use super::actions::select_action::SelectAction;
use super::files_view_component::FileViewActions;

#[derive(Properties, PartialEq)]
pub struct NodeListHeaderProps {
    pub(crate) is_all_nodes_selected: bool,
    pub(crate) action_callback: Callback<FileViewActions>
}

#[function_component(NodeListHeader)]
pub fn node_list_header(props: &NodeListHeaderProps) -> Html {
    let on_header_select_action = {
        let action_callback = props.action_callback.clone();
        Callback::from(move |selection: bool| {
            action_callback.emit(FileViewActions::FileNodesAllSelectionChanged(selection));
        })
    };

    html!{
        <div class="node-list-header">
            <div class="node-list-header__actions">
                <SelectAction is_selected={props.is_all_nodes_selected} on_selection={on_header_select_action} />
            </div>
            <div class="node-list-header__title">
                {"Name"}
            </div>
            <div class="node-list-header__title">
                {"Size"}
            </div>
            <div class="node-list-header__title">
                {"Last Modified"}
            </div>
        </div>
    }
}
