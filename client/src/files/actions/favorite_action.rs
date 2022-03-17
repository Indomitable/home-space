use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub(crate) struct FavoriteActionProps {
    pub(crate) is_favorite: bool,
    pub(crate) on_favorite: Callback<bool>
}

#[function_component(FavoriteAction)]
pub(crate) fn favorite_action(props: &FavoriteActionProps) -> Html {
    let is_favorite = use_state(|| props.is_favorite);

    let onclick = {
        let is_favorite = is_favorite.clone();
        let on_favorite = props.on_favorite.clone();
        Callback::from(move |mouse_event: MouseEvent| {
            let favorite = !*is_favorite;
            is_favorite.set(favorite);
            on_favorite.emit(favorite);
            mouse_event.stop_propagation();
        })
    };

    html!{
        if *is_favorite {
            <span class="icon-outlined file-item-action file-item-action--visible" {onclick}>{"star"}</span>
        } else {
            <span class="icon-outlined file-item-action" {onclick}>{"star_border"}</span>
        }
    }
}