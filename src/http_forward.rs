use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> io::Result<()> {
    let local_port = 8888; // 本地监听端口

    let listener = TcpListener::bind(("0.0.0.0", local_port)).await?;
    println!("HTTP Proxy listening on port {}...", local_port);

    loop {
        let (inbound, _) = listener.accept().await?;

        tokio::spawn(async move {
            if let Err(e) = handle_connection(inbound).await {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
}

async fn handle_connection(mut inbound: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 1024];
    let bytes_read = inbound.read(&mut buffer).await?;

    if bytes_read == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    println!("Received request: \n{}", request);

    // 检查是否为 CONNECT 请求
    if request.starts_with("CONNECT") {
        // 从 CONNECT 请求中解析目标地址
        if let Some(target_address) = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
        {
            println!("CONNECT target: {}", target_address);

            // 连接到目标服务器
            let mut outbound = TcpStream::connect(target_address).await?;

            // 向客户端发送 HTTP 200 响应，表示连接已建立
            let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
            inbound.write_all(response.as_bytes()).await?;

            // 双向转发数据
            let (mut ri, mut wi) = io::split(inbound);
            let (mut ro, mut wo) = io::split(outbound);

            let client_to_proxy = tokio::spawn(async move {
                io::copy(&mut ri, &mut wo).await.ok();
            });

            let proxy_to_client = tokio::spawn(async move {
                io::copy(&mut ro, &mut wi).await.ok();
            });

            let _ = tokio::try_join!(client_to_proxy, proxy_to_client);
        } else {
            eprintln!("Invalid CONNECT request.");
        }

        return Ok(());
    }

    // 处理非 CONNECT 请求
    let target_address =
        if let Some(host_line) = request.lines().find(|line| line.starts_with("Host: ")) {
            host_line.trim_start_matches("Host: ").trim().to_string()
        } else {
            eprintln!("No Host header found in request.");
            return Ok(());
        };

    println!("Forwarding to target: {}", target_address);

    let mut outbound = TcpStream::connect(target_address).await?;

    // 转发请求数据到目标服务器
    outbound.write_all(&buffer[..bytes_read]).await?;

    // 双向转发数据
    let (mut ri, mut wi) = io::split(inbound);
    let (mut ro, mut wo) = io::split(outbound);

    let client_to_proxy = tokio::spawn(async move {
        io::copy(&mut ri, &mut wo).await.ok();
    });

    let proxy_to_client = tokio::spawn(async move {
        io::copy(&mut ro, &mut wi).await.ok();
    });

    let _ = tokio::try_join!(client_to_proxy, proxy_to_client);

    Ok(())
}
