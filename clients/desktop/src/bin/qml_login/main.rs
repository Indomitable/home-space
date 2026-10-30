//! A prototype of the login screen in Qt Quick, through qtbridge. It only signs in and shows
//! who is signed in; the iced app keeps its own login screen for now.

use std::sync::OnceLock;

use qtbridge::{QApp, QmlObject, qobject};
use tokio::runtime::{Builder, Runtime};

use api_client::Client;

const APP_TITLE: &str = "Home Space";
const DEFAULT_SERVER: &str = "http://127.0.0.1:8080";

/// The runtime the requests run on, off the Qt thread.
fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("the tokio runtime starts")
    })
}

pub struct Login {
    server: String,
    username: String,
    busy: bool,
    error: String,
    signed_in_as: String,
}

impl Default for Login {
    fn default() -> Self {
        Self {
            server: DEFAULT_SERVER.to_owned(),
            username: String::new(),
            busy: false,
            error: String::new(),
            signed_in_as: String::new(),
        }
    }
}

#[qobject(Singleton, ConvertToCamelCase)]
impl Login {
    qproperty!("server", Member = server, Notify = server_changed);
    qproperty!("username", Member = username, Notify = username_changed);
    qproperty!("busy", Member = busy, Notify = busy_changed);
    qproperty!("error", Member = error, Notify = error_changed);
    qproperty!(
        "signedInAs",
        Member = signed_in_as,
        Notify = signed_in_as_changed
    );

    #[qsignal]
    fn server_changed(&mut self);
    #[qsignal]
    fn username_changed(&mut self);
    #[qsignal]
    fn busy_changed(&mut self);
    #[qsignal]
    fn error_changed(&mut self);
    #[qsignal]
    fn signed_in_as_changed(&mut self);

    /// Signs in with the current server and username; the password is only passed along.
    #[qslot]
    fn sign_in(&mut self, password: String) {
        if self.busy {
            return;
        }
        self.set_busy(true);
        self.set_error(String::new());
        let server = self.server.trim().trim_end_matches('/').to_owned();
        let username = self.username.trim().to_owned();
        let handle = self.get_thread_handle();
        runtime().spawn(async move {
            let result = Client::new(server).login(&username, &password).await;
            handle.invoke_mut(move |login| {
                login.set_busy(false);
                match result {
                    Ok(me) => {
                        login.signed_in_as = me.username;
                        login.signed_in_as_changed();
                    }
                    Err(err) => login.set_error(err.to_string()),
                }
            });
        });
    }

    #[qslot]
    fn sign_out(&mut self) {
        self.signed_in_as.clear();
        self.signed_in_as_changed();
    }

    fn set_busy(&mut self, busy: bool) {
        self.busy = busy;
        self.busy_changed();
    }

    fn set_error(&mut self, error: String) {
        self.error = error;
        self.error_changed();
    }
}

fn main() {
    let code = QApp::new()
        .application_name(APP_TITLE)
        .register::<Login>()
        .load_qml(include_bytes!("Main.qml"))
        .run();
    std::process::exit(code);
}
