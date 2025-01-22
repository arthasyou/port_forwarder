use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> tokio::io::Result<()> {
    let local_port = 8888; // 本地监听端口
    let ip_pool = Arc::new(Mutex::new(initialize_ip_pool()));

    let listener = TcpListener::bind(("0.0.0.0", local_port)).await?;
    println!("Proxy listening on port {}...", local_port);

    loop {
        let (inbound, _) = listener.accept().await?;
        let ip_pool = Arc::clone(&ip_pool);

        tokio::spawn(async move {
            if let Err(e) = forward_to_proxy(inbound, ip_pool).await {
                eprintln!("Error handling connection: {}", e);
            }
        });
    }
}

fn initialize_ip_pool() -> VecDeque<(String, u16)> {
    VecDeque::from(vec![
        ("127.0.0.1".to_string(), 7890),
        ("127.0.0.1".to_string(), 7891),
        ("127.0.0.1".to_string(), 7892),
    ])
}

async fn forward_to_proxy(
    inbound: TcpStream,
    ip_pool: Arc<Mutex<VecDeque<(String, u16)>>>,
) -> tokio::io::Result<()> {
    println!("ip_pool: {:?}", &ip_pool);
    // 从 IP 池中选择目标代理
    let (proxy_ip, proxy_port) = {
        let mut pool = ip_pool.lock().unwrap();
        pool.pop_front().unwrap_or(("127.0.0.1".to_string(), 7890))
    };

    println!("Forwarding to proxy: {}:{}", proxy_ip, proxy_port);

    // 连接到选定的下游代理
    let outbound = TcpStream::connect((proxy_ip.as_str(), proxy_port)).await?;

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

    // 将目标代理重新放回 IP 池
    let mut pool = ip_pool.lock().unwrap();
    pool.push_back((proxy_ip, proxy_port));

    Ok(())
}
