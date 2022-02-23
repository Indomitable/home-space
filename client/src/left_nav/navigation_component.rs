use yew::prelude::*;
use yew_router::prelude::*;

use crate::router::AppRoute;

#[function_component(LeftNavigation)]
pub fn left_navigation() -> Html {
    html!{
        <nav class="left-nav">
            <ul>
                <li class="left-nav-item">
                    <Link<AppRoute> classes={classes!("left-nav-link")} to={AppRoute::FileList{parent_id: 0}}>
                        <span class="icon-filled">{"home"}</span>
                        <span>{"My Files"}</span>
                    </Link<AppRoute>>
                </li>
                <li class="left-nav-item">
                    <Link<AppRoute> classes={classes!("left-nav-link")} to={AppRoute::Favorites}>
                        <span class="icon-filled">{"favorite"}</span>
                        <span>{"Favorites"}</span>
                    </Link<AppRoute>>
                </li>
                <li class="left-nav-item">
                    <Link<AppRoute> classes={classes!("left-nav-link")} to={AppRoute::Recent}>
                        <span class="icon-filled">{"history"}</span>
                        <span>{"Recent"}</span>
                    </Link<AppRoute>>
                </li>
                <li class="left-nav-item">
                    <Link<AppRoute> classes={classes!("left-nav-link")} to={AppRoute::Shared}>
                        <span class="icon-filled">{"share"}</span>
                        <span>{"Shared"}</span>
                    </Link<AppRoute>>
                </li>
                <li class="left-nav-item">
                    <Link<AppRoute> classes={classes!("left-nav-link")} to={AppRoute::Trash}>
                        <span class="icon-filled">{"restore_from_trash"}</span>
                        <span>{"Trash"}</span>
                    </Link<AppRoute>>
                </li>
            </ul>
        </nav>
    }
}
