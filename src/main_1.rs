// use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let local_port = 8888; // 本地监听端口

    let listener = TcpListener::bind(("0.0.0.0", local_port)).await?;
    println!("Proxy listening on port {}...", local_port);

    loop {
        let (inbound, _) = listener.accept().await?;

        tokio::spawn(async move {
            if let Err(e) = forward_to_proxy(inbound).await {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
}

async fn forward_to_proxy(inbound: TcpStream) -> tokio::io::Result<()> {
    // 连接到下游代理（7890端口）
    let outbound = TcpStream::connect("127.0.0.1:7890").await?;

    // 双向转发数据
    let (mut ri, mut wi) = tokio::io::split(inbound);
    let (mut ro, mut wo) = tokio::io::split(outbound);

    let client_to_proxy = tokio::spawn(async move {
        tokio::io::copy(&mut ri, &mut wo).await.ok();
    });

    let proxy_to_client = tokio::spawn(async move {
        tokio::io::copy(&mut ro, &mut wi).await.ok();
    });

    let _ = tokio::try_join!(client_to_proxy, proxy_to_client);
    Ok(())
}
