use yew::prelude::*;

#[function_component(LeftNavigation)]
pub fn left_navigation() -> Html {
    html!{
        <nav class="left-nav">
            <ul>
                <li>
                    <a class="left-nav-link">
                        <span class="icon">{"home"}</span>
                        <span>{"My Files"}</span>
                    </a>
                </li>
                <li>
                    <a class="left-nav-link">
                        <span class="icon">{"favorite"}</span>
                        <span>{"Favorites"}</span>
                    </a>
                </li>
                <li>
                    <a class="left-nav-link">
                        <span class="icon">{"history"}</span>
                        <span>{"Recent"}</span>
                    </a>
                </li>
                <li>
                    <a class="left-nav-link">
                        <span class="icon">{"share"}</span>
                        <span>{"Shared"}</span>
                    </a>
                </li>
                <li>
                    <a class="left-nav-link">
                        <span class="icon">{"restore_from_trash"}</span>
                        <span>{"Trash"}</span>
                    </a>
                </li>
            </ul>
        </nav>
    }
}
