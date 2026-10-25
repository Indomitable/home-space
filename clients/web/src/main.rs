mod app;
mod events;
mod format;
mod listing;
mod pages;
mod session;
mod upload;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
