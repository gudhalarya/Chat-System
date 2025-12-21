use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::{net::{TcpListener, TcpStream}, sync::{Mutex, mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel}}, time::Instant};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;

type Tx = UnboundedSender<Message>;
type Rx = UnboundedReceiver<Message>;
const MAX_ROOM_SIZE:usize = 15;
type Roomtype = Arc<Mutex<HashMap<String,RoomDs>>>;
struct RoomDs{
    id:String,
    is_open:bool,
    created_at:Instant,
    last_active:Instant,
    clients:Vec<Tx>,
}

//room logic starts here ----MF
async fn create_room(roomtype:Roomtype)->String{
    let mut roomtype = roomtype.lock().await;
    let id = Uuid::new_v4().to_string();
    let now = Instant::now();
    let room = RoomDs{
        id:id.clone(),
        created_at:now,
        is_open:true,
        last_active:now,
        clients:Vec::new(),
    };
    roomtype.insert(id.clone(), room);
    id
}

async fn join_room(roomtype:Roomtype,tx:Tx,room_id:&str)->Result<(),&'static str>{
    let mut roomtype = roomtype.lock().await;
    let room = roomtype.get_mut(room_id).ok_or("Room is not found")?;
    if !room.is_open{
        return Err("Room is not open");
    }
    if room.clients.len()>=MAX_ROOM_SIZE{
        return Err("Room is already full");
    }
    room.clients.push(tx);
    room.last_active=Instant::now();
    Ok(())
}

//this is where the main server is ----Fuck you bitch 
#[tokio::main]
async fn main ()->Result<()>{
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    let roomtype :Roomtype = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let (stream,peer) = listener.accept().await?;
        let roomtype = roomtype.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream,peer).await {
                eprintln!("Error is : {}",e);
            }
        });
    }
}

//this is where the fucking biggest function lies
async fn handle_client(stream:TcpStream,peer:Roomtype)->Result<()>{
    let ws = accept_async(stream).await?;
    let (mut write,mut read) = ws.split();
    let (tx,rx):(Tx,Rx) = unbounded_channel();
    let write_task = tokio::spawn(async move{
        while let Some(msg) = rx.recv().await{
            if write.send(msg).await.is_err(){
                break;
            }
        }
    });


    //now here we will check wether the firt message is create or join 
    let first_text = match read.next().await{
        Some(Ok(Message::Text(text)))=>text,
        _>return Ok(())
    };
    let first_msg = 
}

