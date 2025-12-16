use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;

//this is where the main program starts 
const PORT :&str = "127.0.0.1:8080";
#[tokio::main]
async fn main (){
    let listener = TcpListener::bind(PORT).await.expect("Could not find the port");
    println!("Listener started on : {}",PORT);
    while let Ok((stream,addr)) = listener.accept().await {
        println!("Connection started at : {}",addr);

        tokio::spawn(async move{
            let ws_stream = match accept_async(stream).await {
                Ok(ws)=>ws,
                Err(e)=>{
                    eprintln!("Websocket error : {}",e.to_string());
                    return;
                }
            };
            let (mut write,mut read)= ws_stream.split();
            while let Some(msg) =read.next().await  {
                let msg = match msg {
                    Ok(m)=>m,
                    Err(_)=>break,
                };
                if msg.is_text(){
                    write.send(msg).await.unwrap();
                }
            }
            println!("Connection closed : {}",addr);
        });
    }
}