use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, bail};
use futures_util::StreamExt;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;

// ---------- TYPES ----------

type Tx = UnboundedSender<Message>;
type Rx = UnboundedReceiver<Message>;
type Rooms = Arc<Mutex<HashMap<String, RoomStruct>>>;

const MAX_ROOM_SIZE: usize = 15;

// ---------- ROOM STRUCT ----------

struct RoomStruct {
    id: String,
    clients: Vec<Tx>,
}

// ---------- MAIN ----------

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("✅ Server started at ws://{}", addr);

    let rooms: Rooms = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (stream, peer) = listener.accept().await?;
        println!("🔌 New connection from {}", peer);

        let rooms = rooms.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, rooms).await {
                eprintln!("❌ Client error: {}", e);
            }
        });
    }
}

// ---------- CLIENT HANDLER ----------

async fn handle_client(stream: TcpStream, rooms: Rooms) -> Result<()> {
    let ws = accept_async(stream).await?;
    let (_write, mut read) = ws.split();

    let (tx, _rx): (Tx, Rx) = unbounded_channel();

    // ---- STEP 1: CLIENT SENDS ROOM ID ----
    let room_id = match read.next().await {
        Some(Ok(Message::Text(room_id))) => room_id,
        _ => {
            println!("❌ Client did not send room ID");
            return Ok(());
        }
    };

    // ---- STEP 2: JOIN ROOM ----
    join_room(room_id.clone(), tx, rooms).await?;
    println!("👥 Client joined room {}", room_id);

    // ---- STOP HERE (rooms only phase) ----
    Ok(())
}

// ---------- ROOM LOGIC ----------

async fn create_room(rooms: Rooms) -> Result<String> {
    let mut rooms = rooms.lock().await;

    let room_id = Uuid::new_v4().to_string();
    let room = RoomStruct {
        id: room_id.clone(),
        clients: Vec::new(),
    };

    rooms.insert(room_id.clone(), room);
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
