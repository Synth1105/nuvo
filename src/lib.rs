mod channel;

use std::net::SocketAddr;
use std::sync::OnceLock;

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn get_runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime")
    })
}

pub struct Receiver {
    inner: channel::Receiver,
}

pub struct Sender {
    inner: channel::Session,
}

pub struct ReceiverBuilder {
    port: u16,
    password: Option<String>,
}

pub struct SenderBuilder {
    ip: String,
    port: u16,
    password: String,
}

impl ReceiverBuilder {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            password: None,
        }
    }

    pub fn password(mut self, password: &str) -> Self {
        self.password = Some(password.to_string());
        self
    }

    pub async fn listen(self) -> std::io::Result<Receiver> {
        let inner = channel::rx(self.port, self.password.as_deref()).await?;
        Ok(Receiver { inner })
    }

    pub fn listen_blocking(self) -> std::io::Result<Receiver> {
        get_runtime().block_on(self.listen())
    }
}

impl SenderBuilder {
    pub fn new(ip: &str, port: u16, password: &str) -> Self {
        Self {
            ip: ip.to_string(),
            port,
            password: password.to_string(),
        }
    }

    pub async fn connect(self) -> std::io::Result<Sender> {
        let inner = channel::tx(&self.ip, self.port, &self.password).await?;
        Ok(Sender { inner })
    }

    pub fn connect_blocking(self) -> std::io::Result<Sender> {
        get_runtime().block_on(self.connect())
    }
}

impl Receiver {
    pub async fn accept(&self) -> std::io::Result<Sender> {
        let inner = self.inner.accept().await?;
        Ok(Sender { inner })
    }

    pub fn accept_blocking(&self) -> std::io::Result<Sender> {
        get_runtime().block_on(self.accept())
    }
}

impl Sender {
    pub fn peer_addr(&self) -> SocketAddr {
        self.inner.peer_addr()
    }

    pub async fn send(&mut self, payload: &[u8]) -> std::io::Result<()> {
        self.inner.send(payload).await
    }

    pub fn send_blocking(&mut self, payload: &[u8]) -> std::io::Result<()> {
        get_runtime().block_on(self.send(payload))
    }

    pub async fn recv(&mut self) -> std::io::Result<Vec<u8>> {
        self.inner.recv().await
    }

    pub fn recv_blocking(&mut self) -> std::io::Result<Vec<u8>> {
        get_runtime().block_on(self.recv())
    }
}

pub fn receiver(port: u16) -> ReceiverBuilder {
    ReceiverBuilder::new(port)
}

pub fn sender(ip: &str, port: u16, password: &str) -> SenderBuilder {
    SenderBuilder::new(ip, port, password)
}
