use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use tokio::{net::TcpListener, sync::{Mutex, mpsc::{UnboundedReceiver, UnboundedSender}}, task::id, time::Instant};
use tokio_tungstenite::tungstenite::{Message, handshake::server::ErrorResponse};
use uuid::Uuid;
use serde::{Deserialize,Serialize};

type Tx = UnboundedSender<Message>;
type Rx = UnboundedReceiver<Message>;
type Roomtype = Arc<Mutex<HashMap<String,RoomDs>>>;
const MAX_ROOM_SIZE :usize= 15;


struct RoomDs{
    id:String,
    created_at:Instant,
    last_active:Instant,
    is_open:bool,
    clients:Vec<Clients>,
}

#[derive(Clone)]
struct Clients{
    id:Uuid,
    username:String,
    tx:Tx,
}

//json protocol is here you bitch 
//from clients to server -------------->

#[derive(Debug,Deserialize)]
#[serde(tag="type ")]
enum ClinetsMessage{
    #[serde(rename="hello")]
    Hello {username:String},

    #[serde(rename="create_room")]
    CreateRoom ,

    #[serde(rename="join_room")]
    JoinRoom {room_id:String},

    #[serde(rename="message")]
    Message {text:String},
}

// Server → Client
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "welcome")]
    Welcome { user_id: String },

    #[serde(rename = "room_created")]
    RoomCreated { room_id: String },

    #[serde(rename = "joined_room")]
    JoinedRoom { room_id: String },

    #[serde(rename = "room_message")]
    RoomMessage {
        room_id: String,
        from: String,
        text: String,
    },

    #[serde(rename = "error")]
    Error { message: String },
}

//room logic is written here you fool
async fn create_room(roomtype:Roomtype)->String{
    let room_id=Uuid::new_v4().to_string();
    let mut roomtype = roomtype.lock().await;
    let now = Instant::now();
    let room = RoomDs{
        id:room_id.clone(),
        is_open:true,
        last_active:now,
        created_at:now,
        clients:Vec::new(),
    };
    roomtype.insert(room_id.clone(), room);
    room_id
}

async fn join_room(client:Clients,room_id:&str,roomtype:Roomtype)->Result<(),&'static str>{
    let mut roomtype = roomtype.lock().await;
    let room = roomtype.get_mut(room_id).ok_or("room is not found")?;
    if !room.is_open{
        return Err("room is not open");
    }
    if room.clients.len()>=MAX_ROOM_SIZE{
        return Err("Room is full");
    }
    room.clients.push(client);
    room.last_active=Instant::now();
    Ok(())
}

async fn leave_room(room_id:&str,roomtype:Roomtype,user_id:&Uuid){
    let mut roomstype = roomtype.lock().await;
    if let Some(room) = roomstype.get_mut(room_id) {
        room.clients.retain(|c|&c.id!=user_id);
        room.last_active=Instant::now();
    }
}

async fn cleanup_empty_room(roomtype:Roomtype){
    let mut roomtype = roomtype.lock().await;
    roomtype.retain(|_,room|!room.clients.is_empty());
}

async fn broadcast_to_rooms(roomtype:Roomtype,room_id:&str,msg:Message){
    let roomtype = roomtype.lock().await;
    if let Some(room)=roomtype.get(room_id){
        for client in &room.clients{
            let _ = client.tx.send(msg.clone());
        }
    }
}

//this is where the fn will be added ----later bitch 
#[tokio::main]
async fn main ()->Result<()>{
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    let roomtype:Roomtype = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let (stream,peer) = listener.accept().await?;
        let roomtype = roomtype.clone();
        tokio::spawn(async move {
            if let Err(e)=handle_client(stream,roomtype){
                eprintln!("Error is :{}",e);
            }
        });
    }
}


