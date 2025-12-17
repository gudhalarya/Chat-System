use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use tokio::{net::TcpListener, sync::{Mutex, mpsc::{UnboundedReceiver, UnboundedSender}}};
use tokio_tungstenite::tungstenite::Message;

type Tx= UnboundedSender<Message>;
type Rx = UnboundedReceiver<Message>;
const MAX_ROOM_SIZE:usize=15;
struct Rooms{
    id:String,
    clients : Vec<Tx>,
}

type RoomsType = Arc<Mutex<HashMap<String,Rooms>>>;

#[tokio::main]
async fn main ()->Result<()>{
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("Serevr started at : {}",addr);
    let rooms :RoomsType = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let (stream,peer) = listener.accept().await?;
        println!("New connection at : {}",peer);
        let rooms = rooms.clone();
        tokio::spawn(async move {
            if let Err(e)=handle_client(stream,rooms).await{
                eprintln!("Error is : {}",e);
            }
        });
    }
}