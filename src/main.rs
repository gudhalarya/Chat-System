use std::time::{Duration, Instant};

use actix::{Actor, StreamHandler};
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, web};
use actix_web_actors::ws;

struct WsConn{
    hb:Instant
}

impl Actor for WsConn{
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb=Instant::now();
        self.start_heartbeat(ctx);
        print!("Client Connected");
    }
    fn stopped(&mut self, _: &mut Self::Context) {
        println!("Client disconnected");
    }
}

impl WsConn{
    fn start_heartbeat(&self,ctx:&mut ws::WebsocketContext<Self>){
        ctx.run_interval(Duration::from_secs(5),|act,ctx|{
            if Instant::now().duration_since(self.hb)>Duration::from_secs(10){
                println!("Heartbeat failed,diconnecting");
                ctx.stop();
                return;
            }
            ctx.ping(b"ping");
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }

            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }

            Ok(ws::Message::Text(text)) => {
                println!("Received: {}", text);
                ctx.text(format!("Echo: {}", text));
            }

            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }

            _ => {}
        }
    }
}


async fn ws_route(req:HttpRequest,stream:web::Payload)->Result<HttpResponse,Error>{
    ws::start(WsConn{hb:Instant::now()}, &req, stream)
}

#[actix_web::main]
async fn main ()->std::io::Result<()>{
    HttpServer::new(||{
        App::new()
        .route("/ws", web::get().to(ws_route))
    })
    .bind(("127.0.0.1",8080))?
    .run()
    .await
}