use yew::prelude::*;

#[function_component(LeftNavigation)]
pub fn left_navigation() -> Html {
    html!{
        <nav class="left-nav">
            <ul>
                <li class="left-nav-item">
                    <a class="left-nav-link">
                        <span class="icon-filled">{"home"}</span>
                        <span>{"My Files"}</span>
                    </a>
                </li>
                <li class="left-nav-item">
                    <a class="left-nav-link">
                        <span class="icon-filled">{"favorite"}</span>
                        <span>{"Favorites"}</span>
                    </a>
                </li>
                <li class="left-nav-item">
                    <a class="left-nav-link">
                        <span class="icon-filled">{"history"}</span>
                        <span>{"Recent"}</span>
                    </a>
                </li>
                <li class="left-nav-item">
                    <a class="left-nav-link">
                        <span class="icon-filled">{"share"}</span>
                        <span>{"Shared"}</span>
                    </a>
                </li>
                <li class="left-nav-item">
                    <a class="left-nav-link">
                        <span class="icon-filled">{"restore_from_trash"}</span>
                        <span>{"Trash"}</span>
                    </a>
                </li>
            </ul>
        </nav>
    }
}
