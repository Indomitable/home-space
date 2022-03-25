use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub(crate) struct SelectActionProps {
    pub(crate) is_selected: bool,
    pub(crate) on_selection: Callback<bool>
}

#[function_component(SelectAction)]
pub(crate) fn select_action(props: &SelectActionProps) -> Html {
    let onclick = {
        let is_selected = props.is_selected.clone();
        let on_selection = props.on_selection.clone();
        Callback::from(move |mouse_event: MouseEvent| {
            on_selection.emit(!is_selected);
            mouse_event.stop_propagation();
        })
    };

    html!{
        if props.is_selected {
            <span class="icon-outlined node-row-action node-row-action--visible" {onclick}>{"check_box"}</span>
        } else {
            <span class="icon-outlined node-row-action" {onclick}>{"check_box_outline_blank"}</span>
        }
    }
}