use std::sync::Arc;

use anyhow::Result;
use futures_util::StreamExt;
use tokio::{net::{TcpListener, TcpStream}, sync::{Mutex, mpsc::{self, UnboundedReceiver, UnboundedSender}}};
use tokio_tungstenite::{accept_async, tungstenite::Message};

type Tx = UnboundedSender<Message>;
type Rx = UnboundedReceiver<Message>;

#[tokio::main]
async fn main ()->Result<()>{
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("Listener is ready at : {}",addr);
    let waiting_client : Arc<Mutex<Option<Tx>>> = Arc::new(Mutex::new(None));
    loop {
        let (stream,peer ) = listener.accept().await?;
        println!("Client connected");
        let waiting_client = waiting_client.clone();
        tokio::spawn(async move{
            if let Err(e) = handle_client(stream,waiting_client).await{
                eprintln!("Error : {}",e);
            }
        });
    }
}

async fn handle_client(stream:TcpStream,waiting_client :Arc<Mutex<Option<Tx>>>)->Result<()>{
    let ws =accept_async(stream).await?;
    let (mut write,mut read) = ws.split();
    let (tx,mut rx) :(Tx,Rx) = mpsc::unbounded_channel();

    //this is whre the pairing of the clients take place 
    let peer_tx={
        let mut waiting = waiting_client.lock().await;
        if let Some(other_tx) = waiting.take(){
            println!("Connected two clients");
            Some(other_tx)
        }else{
            println!("waiting for peer");
            *waiting = Some(tx.clone());
            None
        }
    };
    
    //tasks take place here 
    let write_task = tokio::spawn(async move{
        while let Some(msg) = read.next().await{
            let msg = match msg{
                Ok(m)=>m,
                Err(_)=>break,
            } 
        }
    });
}