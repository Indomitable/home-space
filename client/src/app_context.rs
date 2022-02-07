use yew::prelude::*;

#[derive(Debug, PartialEq, Clone)]
pub struct AuthContext {
    pub is_authencitated: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AppContext {
    pub auth_context: AuthContext
}

#[derive(Properties, Debug, PartialEq)]
pub struct AppContextProviderProps {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(AppContextProvider)]
pub fn app_context_provider(props: &AppContextProviderProps) -> Html {
    let context = AppContext{
        auth_context: AuthContext {
            is_authencitated: false            
        }
    };

    html! {
        <ContextProvider<AppContext> context={context}>
            {props.children.clone()}
        </ContextProvider<AppContext>>
    }
}