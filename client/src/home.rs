use yew::prelude::*;

use crate::header::header_component::Header;
use crate::left_nav::navigation_component::LeftNavigation;

#[derive(Properties, PartialEq)]
pub struct LayoutProps {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(Layout)]
pub fn layout(props: &LayoutProps) -> Html {
    html! {
        <div class="home">
            <Header />
            <div class="home-content">
                <aside>
                    <LeftNavigation />
                </aside>
                <section>
                    { for props.children.iter() }
                </section>
            </div>
        </div>
    }
}
