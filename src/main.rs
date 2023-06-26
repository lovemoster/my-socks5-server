use std::error::Error;

use my_socks5_server::handler;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "0.0.0.0:1080";
    let listener = TcpListener::bind(addr).await?;

    while let Ok((src_stream, src_socket)) = listener.accept().await {
        tokio::spawn(async move {
            let _ = handler(src_stream, src_socket).await;
        });
    }

    Ok(())
}
