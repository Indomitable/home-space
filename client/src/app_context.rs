use yew::prelude::*;

#[derive(Debug, PartialEq, Clone)]
pub struct UserContext {
    // pub user_id: i64,
    // pub user_name: String,
    pub access_token: String
}

#[derive(Debug, PartialEq, Clone)]
pub enum AuthContext {
    NotAuthenticated,
    Authenticated(UserContext)
}


#[derive(Debug, PartialEq, Clone)]
pub struct AppContextInner {
    pub auth_context: AuthContext
}

pub enum AppContextAction {
    Authenticate(UserContext),
    LogOut
}

impl Reducible for AppContextInner {
    type Action = AppContextAction;

    fn reduce(self: std::rc::Rc<Self>, action: Self::Action) -> std::rc::Rc<Self> {
        match action {
            AppContextAction::Authenticate(user_context) => {
                AppContextInner {
                    auth_context: AuthContext::Authenticated(user_context)
                }.into()
            },
            AppContextAction::LogOut => {
                AppContextInner {
                    auth_context: AuthContext::NotAuthenticated
                }.into()
            },
        }
    }
}

pub type AppContext = UseReducerHandle<AppContextInner>;

#[derive(Properties, Debug, PartialEq)]
pub struct AppContextProviderProps {
    #[prop_or_default]
    pub children: Children,
}

#[function_component(AppContextProvider)]
pub fn app_context_provider(props: &AppContextProviderProps) -> Html {
    let context = use_reducer(|| AppContextInner {
        auth_context: AuthContext::NotAuthenticated
    });

    html! {
        <ContextProvider<AppContext> context={context}>
            {props.children.clone()}
        </ContextProvider<AppContext>>
    }
}