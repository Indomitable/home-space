use std::{collections::HashMap, borrow::Cow, fmt::Debug};

use log::debug;
use yew::Callback;


#[derive(Debug, PartialEq, Clone)]
pub struct Dispatcher {
    pub subscribers: HashMap<String, Vec<Subscriber>>
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            subscribers: HashMap::new()
        }
    }

    pub fn subscribe<'a>(&mut self, event: Cow<'a, str>, handler: Callback<String>) -> Subscriber {
        let subscriber = Subscriber {
            event: event.clone().into_owned(),
            handler: handler
        };

        // let b = std::boxed::Box::pin(1);
        // Pin

        match self.subscribers.get_mut(event.as_ref()) {
            Some(subscribers) => {
                subscribers.push(subscriber.clone());
            },
            None => {
                self.subscribers.insert(event.clone().into_owned(), vec![ subscriber.clone() ]);
            }
        }

        return subscriber;
    }


    pub fn publish<'a>(&self, event: Cow<'a, str>, payload: String) {
        if let Some(subscribers) = self.subscribers.get(event.as_ref()) {
            for sub in subscribers {
                sub.handler.emit(payload.clone());
            }
        }
    }

    pub fn unsubscribe(&mut self, subsriber: &Subscriber) {
        match self.subscribers.get_mut(&subsriber.event) {
            Some(subscribers) => {
                for (indx, sub) in subscribers.iter().enumerate() {
                    if *sub == *subsriber {
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
