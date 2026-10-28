use std::future::Future;

use leptos::prelude::*;

/// Runs `fetch` now and whenever a signal it reads changes, handing each result to `apply`.
///
/// A result overtaken by a newer run is dropped, so a slow answer never replaces a fresh one.
/// Call `refetch` on the returned resource to load again.
pub fn load<T, Fut>(
    fetch: impl Fn() -> Fut + 'static,
    apply: impl Fn(T) + 'static,
) -> LocalResource<T>
where
    T: Clone + 'static,
    Fut: Future<Output = T> + 'static,
{
    let resource = LocalResource::new(fetch);
    Effect::new(move |_| {
        if let Some(value) = resource.get() {
            untrack(|| apply(value));
        }
    });
    resource
}
