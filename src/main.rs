use std::{collections::HashMap, sync::Arc};

use tokio::{sync::{Mutex, mpsc::UnboundedSender}, time::Instant};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

type Tx = UnboundedSender<Message>;
type RoomsType = Arc<Mutex<HashMap<String,RoomsDs>>>;
struct RoomsDs{
    id:String,
    created_at:Instant,
    last_activity:Instant,
    is_open:bool,
    max_clients:usize,
    clients:Vec<Tx>,
}

async fn create_rooms(max_clients:usize,rooms:RoomsType)->String{
    let mut roomstype = rooms.lock().await;
    let id  = Uuid::new_v4().to_string();
    let now = Instant::now();
    let rooms = RoomsDs{
        id:id.clone(),
        created_at:now,
        last_activity:now,
        is_open:true,
        max_clients:max_clients,
        clients:Vec::new(),
    };
    roomstype.insert(id.clone(),rooms);
    id
}

async fn join_room(rooms: RoomsType, room_id: &str, tx: Tx) -> Result<(), &'static str> {
    let mut rooms = rooms.lock().await;

    let room = rooms.get_mut(room_id).ok_or("Room not found")?;

    if !room.is_open {
        return Err("Room is closed");
    }

    if room.clients.len() >= room.max_clients {
        return Err("Room is full");
    }

    room.clients.push(tx);
    room.last_activity = Instant::now();

    Ok(())
}

async fn leave_room(rooms: Rooms, room_id: &str, tx: &Tx) {
    let mut rooms = rooms.lock().await;

    if let Some(room) = rooms.get_mut(room_id) {
        room.clients.retain(|client| !client.same_channel(tx));
        room.last_activity = Instant::now();
    }
}

async fn cleanup_empty_rooms(rooms: Rooms) {
    let mut rooms = rooms.lock().await;

    rooms.retain(|_, room| !room.clients.is_empty());
}
