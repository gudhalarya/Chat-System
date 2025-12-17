use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use futures_util::StreamExt;
use tokio::{net::{TcpListener, TcpStream}, sync::{Mutex, mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel}}};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;

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

async fn handle_client(stream:TcpStream,rooms:Rooms)->Result<()>{
    let ws = accept_async(stream).await?;
    let (_write,_read) = ws.split();
    let (tx, _rx) :(Tx,Rx) = unbounded_channel();
    let room_id = create_or_join_room_id(tx,rooms.clone()).await?;
    println!("Client joined the room : {}",room_id);

    Ok(()) 
}

async fn create_or_join_room_id(tx:Tx,rooms:Rooms)->Result<()>{
    let mut rooms = rooms.lock().await;
    let room_id = Uuid::new_v4().to_string();
    let room = RoomsType{
        id:room_id.clone(),
        clients:vec![tx],
    };

    rooms.insert(room_id.clone(),room);
    Ok(room_id)
}