use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub(crate) struct SelectActionProps {
    pub(crate) is_selected: bool,
    pub(crate) on_selection: Callback<bool>
}

#[function_component(SelectAction)]
pub(crate) fn select_action(props: &SelectActionProps) -> Html {
    let is_selected = use_state(|| props.is_selected);

    let onclick = {
        let is_selected = is_selected.clone();
        let on_selection = props.on_selection.clone();
        Callback::from(move |mouse_event: MouseEvent| {
            let selected = !*is_selected;
            is_selected.set(selected);
            on_selection.emit(selected);
            mouse_event.stop_propagation();
        })
    };

    html!{
        if *is_selected {
            <span class="icon-outlined file-item-action file-item-action--visible" {onclick}>{"check_box"}</span>
        } else {
            <span class="icon-outlined file-item-action" {onclick}>{"check_box_outline_blank"}</span>
        }
    }
}