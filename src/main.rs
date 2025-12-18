use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use futures_util::{StreamExt, TryFutureExt};
use tokio::{net::{TcpListener, TcpStream}, sync::{Mutex, mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel}}, time::Instant};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;

type Tx = UnboundedSender<Message>;
type Rx = UnboundedReceiver<Message>;
const MAX_ROOM_SIZE :usize = 15;
type RoomType = Arc<Mutex<HashMap<String,RoomDs>>>;
struct RoomDs{
    id:String,
    clients:Vec<Tx>,
    created_at:Instant,
    last_active:Instant,
    is_open:bool,
}

//this is where the fucking room logic is --------
async fn create_rooms(roomtype:RoomType)->String{
    let mut roomtype = roomtype.lock().await;
    let id = Uuid::new_v4().to_string();
    let now = Instant::now();
    let room = RoomDs{
        id:id.clone(),
        created_at:now,
        last_active:now,
        is_open:true,
        clients:Vec::new(),
    };
    roomtype.insert(id.clone(), room);
    id
}

async fn join_rooms(roomstype:RoomType,room_id:&str,tx:Tx)->Result<(),&'static str>{
    let mut roomstype = roomstype.lock().await;
    let room = roomstype.get_mut(room_id).ok_or("Room is not found")?;
    if !room.is_open{
        return Err("Room is closed");
    }

    if room.clients.len()>MAX_ROOM_SIZE{
        return Err("Room is already full");
    }

    room.clients.push(tx);
    room.last_active= Instant::now();
    Ok(())
}

async fn leave_rooms(roomtype:RoomType,room_id:&str,tx:&Tx){
    let mut roomtype = roomtype.lock().await;
    if let Some(room) = roomtype.get_mut(room_id){
        room.clients.retain(|client|!client.same_channel(tx));
        room.last_active= Instant::now();
    }
}

async fn cleanup_rooms(roomstype:RoomType){
    let mut roomstype = roomstype.lock().await;
    roomstype.retain(|_,room|!room.clients.is_empty());
}

//this is where the logic of fucking hanlde_client is ------fuck you mother fucker
async fn hanlde_client(stream:TcpStream,roomstype:RoomType)->Result<()>{
    let ws = accept_async(stream).await?;
    let (_write,mut read) = ws.split();
    let (tx,_rx) :(Tx,Rx)= unbounded_channel();

    let first_msg = match read.next().await{
        Some(Ok(Message::Text(room_id)))=>room_id,
        _=>return Ok(())
    };
    let room_id = if first_msg=="CREATE"{
        let id = create_rooms(roomstype.clone()).await;
        println!("Room is created id is : {}",id);
        id
    }
    else {
        first_msg
    };

    //now let us help this bitch to join a room
    join_rooms(roomstype.clone(), &room_id, tx.clone()).map_err(|e|anyhow::anyhow!(e));
    println!("Client joined the room : {}",room_id);

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Close(_))=>break,
            Ok(_)=>{},
            Err(_)=>break,
        }
    }

    println!("Client left the room");
    leave_rooms(roomstype.clone(), &room_id, &tx).await;
    cleanup_rooms(roomstype).await;
    Ok(())

}


#[tokio::main]
async fn main ()->Result<()>{
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("The srever has started at : {}",addr);

    let rooms:RoomType = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let (stream,peer) = listener.accept().await?;
        println!("New connection at : {}",peer);
        let rooms = rooms.clone();
        tokio::spawn(async move {
            if let Err(e)=hanlde_client(stream, rooms).await{
                eprintln!("Error is there : {}",e);
            }
        });
    }

}