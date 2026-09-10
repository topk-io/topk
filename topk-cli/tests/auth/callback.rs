use std::time::Duration;

use tokio::net::{TcpListener, TcpStream};

use super::run;

#[tokio::test]
async fn timeout_releases_listener_even_with_a_silent_connection() {
    for connected in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let _connection = if connected {
            Some(TcpStream::connect(addr).await.unwrap())
        } else {
            None
        };
        let error = run::<_, _, ()>(
            listener,
            "test-state".into(),
            Duration::from_millis(50),
            |_| async {
                unreachable!("unexpected callback");
            },
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("timed out"));
        assert!(TcpListener::bind(addr).await.is_ok());
    }
}
