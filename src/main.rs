use std::{net::TcpStream, sync::Arc};

use anyhow::Result;
use tokio::{net::TcpListener, sync::{Mutex, mpsc}};
use tokio_tungstenite::{accept_async, tungstenite::Message};

type Tx = mpsc::UnboundedSender<Message>;
type Rx= mpsc::UnboundedReceiver<Message>;

#[tokio::main]
async fn main ()->Result<()>{
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("the listener is working at : {}",addr);

    let waiting_client = Arc<Mutex<Option<Tx>>>= Arc::new(Mutex::new(None));
    loop {
        let (stream,peer) = listener.accept().await?;
        println!("Client connected : {}",peer);
        let waiting_client = waiting_client.clone();
        tokio::spawn(async move{
            if let Err(e) =handle_client(stream,waiting_client).await{
                eprintln!("Error: {}",e);
            } 
        });
    }
}
async fn handle_client(stream:TcpStream,waiting_client:Arc<Mutex<Option<Tx>>>)->Result<()>{
    let ws = accept_async(stream).await?;
    let (mut write,mut read) = ws.split();
    let (tx,mut rx):(Tx,Rx)= mpsc::unbounded_channel();
}

