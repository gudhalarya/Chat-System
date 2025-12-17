use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use futures_util::{StreamExt, stream};
use tokio::{net::{TcpListener, TcpStream}, sync::{Mutex, mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel}}};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;

type Tx = UnboundedSender<Message>;
type Rx = UnboundedReceiver<Message>;
type Rooms = Arc<Mutex<HashMap<String,Tx>>>;
const MAX_ROOM_SIZE :usize = 15;

struct RoomType{
    id:String,
    clients:Vec<Tx>,
}

#[tokio::main]
async fn main ()->Result<()>{
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("Server started at : {}",addr);

    let rooms :Rooms = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let (stream,peer) = listener.accept().await?;
        println!("New connection at : {}",peer);
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream,rooms).await{
                eprintln!("Error is : {}",e);
            }
        });
    }
}

async fn handle_client(stream:TcpStream,rooms:Rooms)->Result<()>{
    let ws = accept_async(stream).await?;
    let (_write, mut read ) = ws.split();
    let (tx , _rx) :(Tx,Rx) = unbounded_channel();
    let room_id = match read.next().await{
        Some(Ok(Message::Text(room_id)))=>room_id,
        _=>{
            println!("Client did not send any room id at all");
            return Ok(());
        }
    };
    join_room(tx,room_id.clone(),rooms).await?;
    println!("Client joined the room : {}",room_id);
    Ok(())
}

async fn create_room(room:Rooms)->Result<String>{
    let mut rooms = room.lock().await;
    let room_id = Uuid::new_v4().to_string();
    let rooms = RoomType{
        id:room_id.clone(),
        clients:Vec::new()
    };
    rooms.insert(room_id.clone(), rooms);
    Ok(room_id)
}

async fn join_room(room_id: String, tx: Tx, rooms: Rooms) -> Result<()> {
    let mut rooms = rooms.lock().await;

    match rooms.get_mut(&room_id) {
        Some(room) => {
            if room.clients.len() >= MAX_ROOM_SIZE {
                bail!("Room is full");
            }
            room.clients.push(tx);
            Ok(())
        }
        None => {
            bail!("Room does not exist");
        }
    }
}