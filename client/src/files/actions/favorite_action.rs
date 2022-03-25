use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub(crate) struct FavoriteActionProps {
    pub(crate) is_favorite: bool,
    pub(crate) on_favorite: Callback<bool>
}

#[function_component(FavoriteAction)]
pub(crate) fn favorite_action(props: &FavoriteActionProps) -> Html {
    let onclick = {
        let is_favorite = props.is_favorite.clone();
        let on_favorite = props.on_favorite.clone();
        Callback::from(move |mouse_event: MouseEvent| {
            on_favorite.emit(!is_favorite);
            mouse_event.stop_propagation();
        })
    };

    html!{
        if props.is_favorite {
            <span class="icon-outlined node-row-action node-row-action--visible" {onclick}>{"star"}</span>
        } else {
            <span class="icon-outlined node-row-action" {onclick}>{"star_border"}</span>
        }
    }
}