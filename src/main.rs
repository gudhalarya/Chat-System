use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
    time::Instant,
};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use uuid::Uuid;

// ===================== TYPES =====================

type Tx = UnboundedSender<Message>;
type Rx = UnboundedReceiver<Message>;
type Rooms = Arc<Mutex<HashMap<String, Room>>>;

const MAX_ROOM_CLIENTS: usize = 15;

// ===================== ROOM STRUCT =====================

struct Room {
    id: String,
    created_at: Instant,
    last_active: Instant,
    is_open: bool,
    clients: Vec<Tx>,
}

// ===================== JSON PROTOCOL =====================

// Client → Server
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "create_room")]
    CreateRoom,

    #[serde(rename = "join_room")]
    JoinRoom { room_id: String },

    #[serde(rename = "message")]
    Message { text: String },
}

// Server → Client
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "room_created")]
    RoomCreated { room_id: String },

    #[serde(rename = "joined_room")]
    JoinedRoom { room_id: String },

    #[serde(rename = "room_message")]
    RoomMessage { room_id: String, text: String },

    #[serde(rename = "error")]
    Error { message: String },
}

// ===================== MAIN =====================

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("✅ Server running at ws://{}", addr);

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

// ===================== CLIENT HANDLER =====================

async fn handle_client(stream: TcpStream, rooms: Rooms) -> Result<()> {
    let ws = accept_async(stream).await?;
    let (mut write, mut read) = ws.split();

    let (tx, rx): (Tx, Rx) = unbounded_channel();

    // ---- WRITE TASK ----
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write.send(msg).await.is_err() {
                break;
            }
        }
    });

    // ---- FIRST MESSAGE (CREATE / JOIN) ----
    let first_text = match read.next().await {
        Some(Ok(Message::Text(text))) => text,
        _ => return Ok(()),
    };

    let first_msg: ClientMessage = match serde_json::from_str(&first_text) {
        Ok(msg) => msg,
        Err(_) => {
            send_error(&tx, "Invalid JSON").await;
            return Ok(());
        }
    };

    let room_id = match first_msg {
        ClientMessage::CreateRoom => {
            let id = create_room(rooms.clone()).await;
            send_json(
                &tx,
                ServerMessage::RoomCreated {
                    room_id: id.clone(),
                },
            )
            .await;
            id
        }

        ClientMessage::JoinRoom { room_id } => {
            join_room(&room_id, rooms.clone(), tx.clone())
                .map_err(|e| anyhow::anyhow!(e))?;

            send_json(
                &tx,
                ServerMessage::JoinedRoom {
                    room_id: room_id.clone(),
                },
            )
            .await;
            room_id
        }

        _ => {
            send_error(&tx, "Invalid first message").await;
            return Ok(());
        }
    };

    println!("👥 Client joined room {}", room_id);

    // ---- READ LOOP (MESSAGING) ----
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(ClientMessage::Message { text }) =
                    serde_json::from_str::<ClientMessage>(&text)
                {
                    let server_msg = ServerMessage::RoomMessage {
                        room_id: room_id.clone(),
                        text,
                    };

                    broadcast_to_room(
                        &room_id,
                        rooms.clone(),
                        Message::Text(serde_json::to_string(&server_msg)?),
                    )
                    .await;
                }
            }
            Ok(Message::Close(_)) => break,
            Err(_) => break,
            _ => {}
        }
    }

    // ---- CLEANUP ----
    println!("❌ Client left room {}", room_id);
    leave_room(&room_id, rooms.clone(), &tx).await;
    cleanup_empty_rooms(rooms).await;

    write_task.abort();
    Ok(())
}

// ===================== ROOM LOGIC =====================

async fn create_room(rooms: Rooms) -> String {
    let id = Uuid::new_v4().to_string();
    let now = Instant::now();

    let room = Room {
        id: id.clone(),
        created_at: now,
        last_active: now,
        is_open: true,
        clients: Vec::new(),
    };

    rooms.lock().await.insert(id.clone(), room);
    id
}

async fn join_room(room_id: &str, rooms: Rooms, tx: Tx) -> Result<(), &'static str> {
    let mut rooms = rooms.lock().await;
    let room = rooms.get_mut(room_id).ok_or("Room not found")?;

    if !room.is_open {
        return Err("Room is closed");
    }

    if room.clients.len() >= MAX_ROOM_CLIENTS {
        return Err("Room is full");
    }

    room.clients.push(tx);
    room.last_active = Instant::now();
    Ok(())
}

async fn leave_room(room_id: &str, rooms: Rooms, tx: &Tx) {
    let mut rooms = rooms.lock().await;
    if let Some(room) = rooms.get_mut(room_id) {
        room.clients.retain(|c| !c.same_channel(tx));
        room.last_active = Instant::now();
    }
}

async fn cleanup_empty_rooms(rooms: Rooms) {
    let mut rooms = rooms.lock().await;
    rooms.retain(|_, room| !room.clients.is_empty());
}

// ===================== HELPERS =====================

async fn broadcast_to_room(room_id: &str, rooms: Rooms, msg: Message) {
    let rooms = rooms.lock().await;
    if let Some(room) = rooms.get(room_id) {
        for client in &room.clients {
            let _ = client.send(msg.clone());
        }
    }
}

async fn send_json(tx: &Tx, msg: ServerMessage) {
    let _ = tx.send(Message::Text(
        serde_json::to_string(&msg).unwrap(),
    ));
}

async fn send_error(tx: &Tx, message: &str) {
    send_json(
        tx,
        ServerMessage::Error {
            message: message.to_string(),
        },
    )
    .await;
}
