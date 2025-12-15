use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use futures_util::{StreamExt, SinkExt};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080")
        .await
        .expect("Failed to bind");

    println!("🚀 WebSocket server running on ws://127.0.0.1:8080");

    while let Ok((stream, addr)) = listener.accept().await {
        println!("New connection from {}", addr);

        tokio::spawn(async move {
            let ws_stream = match accept_async(stream).await {
                Ok(ws) => ws,
                Err(e) => {
                    eprintln!("WebSocket error: {}", e);
                    return;
                }
            };

            let (mut write, mut read) = ws_stream.split();

            while let Some(msg) = read.next().await {
                let msg = match msg {
                    Ok(m) => m,
                    Err(_) => break,
                };

                if msg.is_text() {
                    
                    write.send(msg).await.unwrap();
                }
            }

            println!("Connection closed: {}", addr);
        });
    }
}
