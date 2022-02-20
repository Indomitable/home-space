use yew::prelude::*;

use crate::header::header_component::Header;
use crate::left_nav::navigation_component::LeftNavigation;
use crate::files::files_view_component::FilesView;

#[function_component(Home)]
pub fn home() -> Html {
    

    html! {
        <div class="home">
            <Header />
            <div class="home-content">
                <aside>
                    <LeftNavigation />
                </aside>
                <section>
                    <FilesView />
                </section>
            </div>
        </div>
    }
}
