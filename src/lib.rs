//! Nuvo: A simple and secure client-server communication library.
//!
//! This crate provides a high-level API for establishing secure TCP connections
//! between a `Receiver` (server) and a `Sender` (client). It supports both
//! asynchronous (Tokio-based) and synchronous (blocking) operations.
//!
//! # Examples
//!
//! ```no_run
//! use nuvo::prelude::*;
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> std::io::Result<()> {
//!     let rx = receiver(8080).password("mypass").listen().await?;
//!     // ... accept and communicate
//!     Ok(())
//! }
//! ```

mod channel;

/// The prelude module for Nuvo.
///
/// This module re-exports the most commonly used types and functions for
/// convenient access.
pub mod prelude {
    pub use crate::internal::{
        receiver, sender, Receiver, ReceiverBuilder, Sender, SenderBuilder,
    };
}

use std::net::SocketAddr;
use std::sync::OnceLock;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// Internal helper to get or initialize a global Tokio runtime for blocking calls.
fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime")
    })
}

mod internal {
    use super::*;

    /// A receiver that listens for incoming connections.
    pub struct Receiver {
        pub(crate) inner: channel::Receiver,
    }

    /// A sender (or accepted session) used to communicate with a peer.
    pub struct Sender {
        pub(crate) inner: channel::Session,
    }

    /// A builder for creating a [`Receiver`].
    pub struct ReceiverBuilder {
        pub(crate) port: u16,
        pub(crate) password: Option<String>,
    }

    /// A builder for creating a [`Sender`].
    pub struct SenderBuilder {
        pub(crate) ip: String,
        pub(crate) port: u16,
        pub(crate) password: String,
    }

    impl ReceiverBuilder {
        /// Creates a new `ReceiverBuilder` for the specified port.
        pub fn new(port: u16) -> Self {
            Self {
                port,
                password: None,
            }
        }

        /// Sets an optional password for authentication.
        pub fn password(mut self, password: &str) -> Self {
            self.password = Some(password.to_string());
            self
        }

        /// Binds to the port and starts listening for connections asynchronously.
        pub async fn listen(self) -> std::io::Result<Receiver> {
            let inner = channel::rx(self.port, self.password.as_deref()).await?;
            Ok(Receiver { inner })
        }

        /// Binds to the port and starts listening for connections synchronously.
        pub fn listen_blocking(self) -> std::io::Result<Receiver> {
            get_runtime().block_on(self.listen())
        }
    }

    impl SenderBuilder {
        /// Creates a new `SenderBuilder` for the specified address and password.
        pub fn new(ip: &str, port: u16, password: &str) -> Self {
            Self {
                ip: ip.to_string(),
                port,
                password: password.to_string(),
            }
        }

        /// Connects to the receiver asynchronously.
        pub async fn connect(self) -> std::io::Result<Sender> {
            let inner = channel::tx(&self.ip, self.port, &self.password).await?;
            Ok(Sender { inner })
        }

        /// Connects to the receiver synchronously.
        pub fn connect_blocking(self) -> std::io::Result<Sender> {
            get_runtime().block_on(self.connect())
        }
    }

    impl Receiver {
        /// Accepts an incoming connection asynchronously.
        pub async fn accept(&self) -> std::io::Result<Sender> {
            let inner = self.inner.accept().await?;
            Ok(Sender { inner })
        }

        /// Accepts an incoming connection synchronously.
        pub fn accept_blocking(&self) -> std::io::Result<Sender> {
            get_runtime().block_on(self.accept())
        }
    }

    impl Sender {
        /// Returns the remote address of the peer.
        pub fn peer_addr(&self) -> SocketAddr {
            self.inner.peer_addr()
        }

        /// Sends a payload to the peer asynchronously.
        pub async fn send(&mut self, payload: &[u8]) -> std::io::Result<()> {
            self.inner.send(payload).await
        }

        /// Sends a payload to the peer synchronously.
        pub fn send_blocking(&mut self, payload: &[u8]) -> std::io::Result<()> {
            get_runtime().block_on(self.send(payload))
        }

        /// Receives a payload from the peer asynchronously.
        pub async fn recv(&mut self) -> std::io::Result<Vec<u8>> {
            self.inner.recv().await
        }

        /// Receives a payload from the peer synchronously.
        pub fn recv_blocking(&mut self) -> std::io::Result<Vec<u8>> {
            get_runtime().block_on(self.recv())
        }
    }

    /// Entry point to build a [`Receiver`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nuvo::prelude::receiver;
    ///
    /// # async fn run() -> std::io::Result<()> {
    /// let rx = receiver(8080).password("secret").listen().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn receiver(port: u16) -> ReceiverBuilder {
        ReceiverBuilder::new(port)
    }

    /// Entry point to build a [`Sender`].
    ///
    /// # Example
    ///
    /// ```no_run
    /// use nuvo::prelude::sender;
    ///
    /// # async fn run() -> std::io::Result<()> {
    /// let mut tx = sender("127.0.0.1", 8080, "secret").connect().await?;
    /// tx.send(b"hello").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn sender(ip: &str, port: u16, password: &str) -> SenderBuilder {
        SenderBuilder::new(ip, port, password)
    }
}
