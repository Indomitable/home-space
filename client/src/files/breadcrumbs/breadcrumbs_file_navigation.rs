use home_space_contracts::files::ParentNode;
use wasm_bindgen::UnwrapThrowExt;
use yew::prelude::*;
use yew_router::prelude::*;

use crate::{router::AppRoute, files::file_repository::load_breadcrumbs};

#[derive(Properties, PartialEq)]
pub struct BreadcumbsFileNavProps {
    pub parent_id: i64,
    pub access_token: String,
}

pub struct BreadcumbsFileNav {
    current_parent_id: i64,
    nodes: Option<Vec<ParentNode>>
}

#[derive(Debug)]
pub enum BreadcumbsMessage {
    FetchParents(i64, String),
    ParentsFetched { parents: Vec<ParentNode> },
    FetchError,
}

impl BreadcumbsFileNav {
    fn load_breadcrumbs(&self, ctx: &Context<Self>) {
        let cb = ctx.link().callback(|x: (i64, String)| BreadcumbsMessage::FetchParents(x.0, x.1));
        cb.emit((ctx.props().parent_id, ctx.props().access_token.clone()));
    }
}

impl Component for BreadcumbsFileNav {
    type Message = BreadcumbsMessage;
    type Properties = BreadcumbsFileNavProps;

    fn create(ctx: &Context<Self>) -> Self {
        let parent_id = ctx.props().parent_id;
        let component = Self {
            current_parent_id: parent_id,
            nodes: None
        };
        if parent_id > 0 {
            // Top parent is hardcoded in the code, no need to fetch
            component.load_breadcrumbs(ctx);
        }
        component
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            BreadcumbsMessage::FetchParents ( id, token ) => {
                let callback = ctx.link().callback_future(|props: (i64, String)| async move {
                    let breadcrumbs = load_breadcrumbs(props.0, &props.1).await;
                    return match breadcrumbs {
                        Ok(nodes) => BreadcumbsMessage::ParentsFetched { parents: nodes },
                        Err(_) => BreadcumbsMessage::FetchError
                    }
                });
                callback.emit((id,  token));
                false
            },
            BreadcumbsMessage::ParentsFetched { parents } => {
                self.nodes = Some(parents);
                true
            },
            BreadcumbsMessage::FetchError => {
                // Todo show error ?!?
                false
            }
        }
    }


    fn changed(&mut self, ctx: &Context<Self>) -> bool {
        let parent_id = ctx.props().parent_id;
        if self.current_parent_id != parent_id {
            // Parent id is changed
            self.load_breadcrumbs(ctx);
            self.current_parent_id = parent_id
        }
        false // Do not render, it will happen in the update
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        if ctx.props().parent_id > 0 {
            match self.nodes {
                Some(ref nodes) => {
                    html! {
                        <nav class="breadcrumbs-nav">
                        {
                            nodes.iter().map(|node: &ParentNode| {
                                html! {
                                    <Breadcrumb node={node.clone()} />   
                                }
                            }).collect::<Html>()
                        }
                        </nav>
                    }
                },
                None => {
                    html!{ <nav class="breadcrumbs-nav"></nav>}
                }
            }
        } else {
            html!{
                <nav class="breadcrumbs-nav">
                    <div class="breadcrumb-item breadcrumb-item-current">
                        <span class="icon-filled">{"home"}</span>
                        <span>{"My Files"}</span>
                    </div>
                </nav>
            }
        }        
    }
}


#[derive(Properties, PartialEq)]
struct BreadcrumbProps {
    node: ParentNode
}

#[function_component(Breadcrumb)]
fn breadcrumb(prop: &BreadcrumbProps) -> Html {
    let ParentNode { id, title } = &prop.node;
    let navigator = use_navigator().unwrap_throw();
    let is_current = if let AppRoute::FileList{parent_id} = use_route().unwrap_throw() {
        *id == parent_id
    } else {
        false
    };
    let onclick = {
        let id = *id;
        Callback::from(move |_| {
            if !is_current {
                navigator.push(&AppRoute::FileList{ parent_id: id });
            }
        })
    };
    html! {
        <div class={classes!("breadcrumb-item", is_current.then(|| Some("breadcrumb-item-current")))} key={*id} {onclick}>
            if *id == 0_i64 {
                <span class="icon-filled">{"home"}</span>
                <span>{"My Files"}</span>
            } else {
                <span>{title.clone()}</span>
            }            
        </div>
    }
}



// #[function_component(BreadcumbsFileNav)]
// pub fn file_actions(props: &BreadcumbsFileNavProps) -> Html {
//     let state = use_state(Vec::<ParentNode>::new);
//     if props.parent_id > 0 && state.len() == 0 {
//         let parent_id = props.parent_id;
//         let token = props.access_token.clone();
//         let state = state.clone();
//         wasm_bindgen_futures::spawn_local(async move {
//             let url = format!("/api/files/parents/{}", parent_id);
//             let reader: ResponseReader = RequestInitBuilder::<()>::new()
//                 .set_method(METHOD_GET)
//                 .set_url(&url)
//                 .set_access_token(&token)
//                 .fetch()
//                 .await
//                 .into();
//             if let Ok(nodes) = reader.as_obj::<Vec<ParentNode>>().await {
//                 state.set(nodes);
//             }
//         });
//     }

//     html! {
//         <ul class="breadcrumbs-nav">
//             if props.parent_id == 0 {
//                 <li class="breadcrumb-item">
//                     <span class="icon-filled">{"home"}</span>
//                     <span>{"My files"}</span>
//                 </li>
//             } else {
//                 {
//                     state.iter().map(|node: &ParentNode| {
//                         html! {
//                             <li class="breadcrumb-item">
//                                 <span class="icon-filled">{"home"}</span>
//                                 <span>{node.title.clone()}</span>
//                             </li>
//                         }
//                     }).collect::<Html>()
//                 }
//             }

//         </ul>
//     }
// }
