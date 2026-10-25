use leptos::{html::Div, prelude::*};
use web_sys::HtmlElement;

use crate::icon::Icon;

const ICON_MORE: &str = "more_horiz";
const POPOVER_AUTO: &str = "auto";

#[derive(Clone, Copy)]
struct MenuPanel(NodeRef<Div>);

fn close(panel: NodeRef<Div>) {
    if let Some(element) = panel.get_untracked() {
        let element: &HtmlElement = &element;
        let _ = element.hide_popover();
    }
}

/// A button that opens a popover list of actions anchored under it.
#[component]
pub fn Menu(
    id: String,
    #[prop(into)] label: String,
    #[prop(optional, into)] text: Option<Signal<String>>,
    #[prop(default = ICON_MORE)] icon: &'static str,
    children: ChildrenFn,
) -> impl IntoView {
    let panel = NodeRef::<Div>::new();
    provide_context(MenuPanel(panel));
    let place = move |event: leptos::ev::MouseEvent| {
        use wasm_bindgen::JsCast;
        let Some(target) = event
            .current_target()
            .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
        else {
            return;
        };
        let rect = target.get_bounding_client_rect();
        let width = web_sys::window()
            .and_then(|window| window.inner_width().ok())
            .and_then(|width| width.as_f64())
            .unwrap_or_default();
        if let Some(element) = panel.get_untracked() {
            let style = HtmlElement::style(&element);
            let _ = style.set_property("top", &format!("{}px", rect.bottom() + 4.0));
            let _ = style.set_property("right", &format!("{}px", width - rect.right()));
            let _ = style.set_property("left", "auto");
        }
    };

    view! {
        <button
            class="button button--ghost button--small menu__trigger"
            aria-label=label
            aria-haspopup="menu"
            popovertarget=id.clone()
            on:click=place
        >
            {move || (!icon.is_empty()).then(|| view! { <Icon name=icon /> })}
            {move || text.map(|text| text.get())}
        </button>
        <div id=id class="menu" role="menu" popover=POPOVER_AUTO node_ref=panel>
            {children()}
        </div>
    }
}

/// One entry of a [`Menu`]; closes the menu after running `on_select`.
#[component]
pub fn MenuItem(
    icon: &'static str,
    #[prop(into)] label: String,
    #[prop(optional)] danger: bool,
    on_select: Callback<()>,
) -> impl IntoView {
    let panel = expect_context::<MenuPanel>().0;
    view! {
        <button
            class="menu__item"
            class:menu__item--danger=danger
            role="menuitem"
            on:click=move |_| {
                close(panel);
                on_select.run(());
            }
        >
            <Icon name=icon />
            {label}
        </button>
    }
}

/// A menu entry that navigates; closes the menu on click.
#[component]
pub fn MenuLink(href: String, children: Children) -> impl IntoView {
    let panel = expect_context::<MenuPanel>().0;
    view! {
        <leptos_router::components::A
            href=href
            exact=true
            attr:class="menu__item"
            attr:role="menuitem"
            on:click=move |_| close(panel)
        >
            {children()}
        </leptos_router::components::A>
    }
}
