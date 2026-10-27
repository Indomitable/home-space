use leptos::{html::Div, prelude::*};
use web_sys::HtmlElement;

use crate::icon::Icon;

const ICON_MORE: &str = "more_horiz";
const POPOVER_AUTO: &str = "auto";
/// Space between the trigger and the panel, and the panel and the window edge.
const MENU_GAP: f64 = 4.0;

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
        let (width, height) = web_sys::window()
            .and_then(|window| {
                let width = window.inner_width().ok()?.as_f64()?;
                let height = window.inner_height().ok()?.as_f64()?;
                Some((width, height))
            })
            .unwrap_or_default();
        if let Some(element) = panel.get_untracked() {
            let style = HtmlElement::style(&element);
            let y = top(rect.top(), rect.bottom(), measure_height(&element), height);
            let _ = style.set_property("top", &format!("{y}px"));
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

/// The panel's height, measured while it is still hidden (before the popover opens).
fn measure_height(element: &HtmlElement) -> f64 {
    let style = element.style();
    let _ = style.set_property("display", "block");
    let _ = style.set_property("visibility", "hidden");
    let height = f64::from(element.offset_height());
    let _ = style.remove_property("display");
    let _ = style.remove_property("visibility");
    height
}

/// Where the panel's top goes: under the trigger, or above it when it would run off the bottom.
fn top(trigger_top: f64, trigger_bottom: f64, panel: f64, viewport: f64) -> f64 {
    let below = trigger_bottom + MENU_GAP;
    if below + panel + MENU_GAP <= viewport {
        below
    } else {
        (trigger_top - MENU_GAP - panel).max(MENU_GAP)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_opens_under_the_trigger_when_it_fits() {
        assert_eq!(top(100.0, 120.0, 200.0, 800.0), 124.0);
    }

    #[test]
    fn menu_opens_above_the_trigger_near_the_bottom() {
        assert_eq!(top(700.0, 720.0, 200.0, 800.0), 496.0);
    }

    #[test]
    fn menu_stays_inside_a_short_window() {
        assert_eq!(top(100.0, 120.0, 400.0, 300.0), MENU_GAP);
    }
}
