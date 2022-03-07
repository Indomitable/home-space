use web_sys::MouseEvent;
use yew::{Properties, Children, function_component, Html, html, Callback};

#[derive(PartialEq)]
pub enum ModalDialogHeader {
    Text(String),
    Html(Html)
}

#[derive(Properties, PartialEq)]
pub struct ModalDialogProps {
    pub header:  ModalDialogHeader,
    pub use_backdrop: Option<bool>,
    pub on_backdrop_click: Option<Callback<MouseEvent>>,
    #[prop_or_default]
    pub children: Children
}

#[function_component(ModalDialog)]
pub fn modal_dialog(props: &ModalDialogProps) -> Html {
    let header = match &props.header {
        ModalDialogHeader::Text(txt) => {
            html!(<span>{txt}</span>)
        },
        ModalDialogHeader::Html(html) => {
            html.clone()
        }
    };
    html! {
        <>
            <div class="modal-dialog">
                <div class="modal-header">{header}</div>
                <div class="modal-body">
                    {for props.children.iter()}
                </div>
            </div>
            if let Some(true) = props.use_backdrop {
                <div class="modal-backdrop" onclick={props.on_backdrop_click.clone()}></div>
            }
        </>
    }
}