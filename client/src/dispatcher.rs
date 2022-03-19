use std::{collections::HashMap, borrow::Cow, fmt::Debug, rc::Rc};

use log::debug;
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen::UnwrapThrowExt;
use yew::Callback;


#[derive(Debug, PartialEq, Clone)]
pub struct Dispatcher {
    pub subscribers: HashMap<String, Vec<Rc<Subscriber>>>
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new()
        }
    }

    pub fn subscribe<'a, TPayload>(&mut self, event: Cow<'a, str>, payload_handler: Callback<TPayload>) -> Rc<Subscriber>
    where TPayload : DeserializeOwned + 'static {
        let handler = Callback::from(move |data: String| {
            let payload = serde_json::from_str::<TPayload>(&data).unwrap_throw();
            payload_handler.emit(payload);
        });
        let subscriber = Rc::new(Subscriber {
            event: event.clone().into_owned(),
            handler: handler
        });

        match self.subscribers.get_mut(event.as_ref()) {
            Some(subscribers) => {
                subscribers.push(Rc::clone(&subscriber));
            },
            None => {
                self.subscribers.insert(event.clone().into_owned(), vec![
                    Rc::clone(&subscriber)
                ]);
            }
        }

        return subscriber;
    }


    pub fn publish<'a, T>(&self, event: Cow<'a, str>, payload: T) 
    where T: Serialize {
        if let Some(subscribers) = self.subscribers.get(event.as_ref()) {
            let data = serde_json::to_string(&payload).unwrap_throw();
            for sub in subscribers {
                sub.handler.emit(data.clone());
            }
        }
    }

    pub fn unsubscribe(&mut self, subsriber: &Rc<Subscriber>) {
        match self.subscribers.get_mut(&subsriber.event) {
            Some(subscribers) => {
                for (indx, sub) in subscribers.iter().enumerate() {
                    if Rc::ptr_eq(sub, subsriber) {
                        debug!("Removed subscriber on index: {}", indx);
                        subscribers.remove(indx);
                        debug!("Subscribers: {}", subscribers.len());
                        return;
                    }
                }
            },
            None => {

            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Subscriber {
    pub event: String,
    pub handler: Callback<String>
}
