use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{
        mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};
use tokio_tungstenite::{accept_async, tungstenite::Message};

type Tx = UnboundedSender<Message>;
type Rx = UnboundedReceiver<Message>;
type Rooms = Arc<Mutex<HashMap<String, Vec<Tx>>>>;

const MAX_ROOM_SIZE: usize = 15;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("✅ Server running at ws://{}", addr);

    let rooms: Rooms = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (stream, peer) = listener.accept().await?;
        println!("🔌 New connection: {}", peer);

        let rooms = rooms.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, rooms).await {
                eprintln!("❌ Client error: {}", e);
            }
        });
    }
}

async fn handle_client(stream: TcpStream, rooms: Rooms) -> Result<()> {
    let ws = accept_async(stream).await?;
    let (mut write, mut read) = ws.split();

    let (tx, mut rx): (Tx, Rx) = unbounded_channel();

    // ---- JOIN ROOM (hardcoded room1 for now) ----
    let room_name = "room1".to_string();

    {
        let mut rooms = rooms.lock().await;
        let room = rooms.entry(room_name.clone()).or_insert_with(Vec::new);

        if room.len() >= MAX_ROOM_SIZE {
            write
                .send(Message::Text("❌ Room full".into()))
                .await?;
            return Ok(());
        }

        room.push(tx.clone());
        println!("👥 Client joined {}", room_name);
    }

    // Task: send messages TO this client
    let write_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if write.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Task: receive messages FROM this client
    let read_task = {
        let rooms = rooms.clone();
        let room_name = room_name.clone();

        tokio::spawn(async move {
            while let Some(Ok(msg)) = read.next().await {
                match msg {
                    Message::Text(_) | Message::Binary(_) => {
                        let rooms = rooms.lock().await;
                        if let Some(room) = rooms.get(&room_name) {
                            for peer in room {
                                let _ = peer.send(msg.clone());
                            }
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }

            // ---- CLEANUP ON DISCONNECT ----
            let mut rooms = rooms.lock().await;
            if let Some(room) = rooms.get_mut(&room_name) {
                room.retain(|peer| !peer.same_channel(&tx));
            }

            println!("🔌 Client left {}", room_name);
        })
    };

    tokio::select! {
        _ = write_task => (),
        _ = read_task => (),
    }

    Ok(())
}
