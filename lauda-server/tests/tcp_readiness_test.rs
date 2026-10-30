use std::net::SocketAddr;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

/// Sonda assíncrona de prontidão TCP: aguarda até que o socket esteja aceitando conexões.
pub async fn wait_for_tcp_ready(addr: SocketAddr) {
    for _ in 0..50 {
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            let _ = stream.shutdown().await;
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("Timeout aguardando prontidão TCP em {addr}");
}

/// Versão parametrizada com resultado explícito para asserções de erro/timeout.
pub async fn try_wait_for_tcp_ready(addr: SocketAddr, max_retries: usize, delay: Duration) -> bool {
    for _ in 0..max_retries {
        if let Ok(mut stream) = TcpStream::connect(addr).await {
            let _ = stream.shutdown().await;
            return true;
        }
        tokio::time::sleep(delay).await;
    }
    false
}

#[tokio::test]
async fn test_tcp_readiness_fails_on_closed_port() {
    // 1. Vincula uma porta efêmera e descarta imediatamente o listener para garantir que a porta está fechada
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let closed_addr = listener.local_addr().unwrap();
    drop(listener);

    // 2. Deve falhar graciosamente retornando false dentro do limite estipulado
    let ready = try_wait_for_tcp_ready(closed_addr, 5, Duration::from_millis(5)).await;
    assert!(!ready, "Porta fechada não deve ser reportada como pronta");
}

#[tokio::test]
async fn test_tcp_readiness_succeeds_promptly_on_listening_server() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let _ = socket.write_all(b"OK").await;
    });

    // Deve conectar e acusar prontidão imediatamente (< 50ms)
    wait_for_tcp_ready(addr).await;

    server_handle.await.unwrap();
}
