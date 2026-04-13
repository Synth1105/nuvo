# Nuvo

Nuvo is a lightweight, secure communication library for Rust, designed for simple client-server messaging. It provides both asynchronous (using Tokio) and synchronous (blocking) APIs for easy integration into various project types.

## Features

- **Simple API**: Easy-to-use `Sender` and `Receiver` builders.
- **Secure**: Basic password authentication and session tokens.
- **Async & Sync**: Full support for `async/await` and traditional blocking calls.
- **Lightweight Protocol**: Custom binary protocol optimized for efficiency.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
nuvo = "0.1.0"
```

## Quick Start

### Asynchronous Example

```rust
use nuvo::{receiver, sender};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Start a receiver
    let rx = receiver(8080).password("secret").listen().await?;

    // Connect with a sender
    let mut tx = sender("127.0.0.1", 8080, "secret").connect().await?;

    // Accept the connection on the receiver side
    let mut rx_session = rx.accept().await?;

    // Send and receive data
    tx.send(b"Hello from sender!").await?;
    let msg = rx_session.recv().await?;
    println!("Received: {:?}", String::from_utf8_lossy(&msg));

    Ok(())
}
```

### Blocking Example

```rust
use nuvo::{receiver, sender};

fn main() -> std::io::Result<()> {
    // Start a receiver
    let rx = receiver(8081).password("secret").listen_blocking()?;

    // In a real application, these would be in different threads/processes
    std::thread::spawn(move || {
        let mut tx = sender("127.0.0.1", 8081, "secret").connect_blocking().unwrap();
        tx.send_blocking(b"Hello!").unwrap();
    });

    let mut rx_session = rx.accept_blocking()?;
    let msg = rx_session.recv_blocking()?;
    println!("Received: {:?}", String::from_utf8_lossy(&msg));

    Ok(())
}
```

## License

LGPL-3.0-only
