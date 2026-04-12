use nuvo::{receiver, sender};

#[tokio::test]
async fn test_rx_with_password_success() {
    let port = 19999;
    let password = "test_password";

    let receiver = receiver(port).password(password).listen().await.unwrap();
    
    let tx_handle = tokio::spawn(async move {
        sender("127.0.0.1", port, password).connect().await.unwrap()
    });

    let session = receiver.accept().await.unwrap();
    tx_handle.await.unwrap();

    assert!(session.peer_addr().ip().is_loopback());
}

#[tokio::test]
async fn test_rx_with_password_reject() {
    let port = 19998;
    let expected_password = "correct_password";
    let wrong_password = "wrong_password";

    let receiver = receiver(port).password(expected_password).listen().await.unwrap();
    
    let tx_handle = tokio::spawn(async move {
        sender("127.0.0.1", port, wrong_password).connect().await
    });

    let accept_result = receiver.accept().await;
    let tx_result = tx_handle.await.unwrap();

    assert!(accept_result.is_err());
    assert!(tx_result.is_err());
}

#[tokio::test]
async fn test_rx_without_password_accepts_any() {
    let port = 19997;
    let password = "any_password";

    let receiver = receiver(port).listen().await.unwrap();
    
    let tx_handle = tokio::spawn(async move {
        sender("127.0.0.1", port, password).connect().await.unwrap()
    });

    let session = receiver.accept().await.unwrap();
    tx_handle.await.unwrap();

    assert!(session.peer_addr().ip().is_loopback());
}