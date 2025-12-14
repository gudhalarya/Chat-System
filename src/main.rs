use std::time::{Duration, Instant};

use actix::Actor;
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