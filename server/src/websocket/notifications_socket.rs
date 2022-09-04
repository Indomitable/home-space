use std::time::Duration;
use actix::{Actor, AsyncContext, StreamHandler};
use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use log::debug;

/// Define HTTP actor
struct NotificationsSocket {
}

impl NotificationsSocket {
    fn heartbeat(&mut self, context: &mut ws::WebsocketContext<Self>) {
        context.run_interval(Duration::from_secs(3), |socket, context| {
            context.ping(b"haha");
        });
    }
}

impl Actor for NotificationsSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
    }
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for NotificationsSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Pong(bin)) => {
                debug!("Received pong, {:?}", bin);
            },
            _ => (),
        }
    }
}

pub(super) async fn index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    ws::start(NotificationsSocket {}, &req, stream)
}
