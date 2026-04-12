use std::io;
use std::net::SocketAddr;

use rand::RngCore;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const MAGIC: [u8; 2] = *b"NV";
const VERSION: u8 = 1;

const TYPE_HELLO: u8 = 1;
const TYPE_ACCEPT: u8 = 2;
const TYPE_REJECT: u8 = 3;
const TYPE_DATA: u8 = 4;

const MAX_PASSWORD_LEN: usize = 1024;
const MAX_TOKEN_LEN: usize = 64;
const TOKEN_LEN: usize = 32;
const MAX_PAYLOAD_LEN: usize = 8 * 1024 * 1024;

pub struct Receiver {
    listener: TcpListener,
    expected_password: Option<String>,
}

pub struct Session {
    stream: TcpStream,
    token: Vec<u8>,
    peer: SocketAddr,
}

pub async fn rx(port: u16, expected_password: Option<&str>) -> io::Result<Receiver> {
    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(addr).await?;
    Ok(Receiver {
        listener,
        expected_password: expected_password.map(|s| s.to_string()),
    })
}

pub async fn tx(ip: &str, port: u16, password: &str) -> io::Result<Session> {
    let addr = (ip, port);
    let mut stream = TcpStream::connect(addr).await?;
    send_hello(&mut stream, password).await?;
    let (token, peer) = read_accept(&mut stream).await?;
    Ok(Session { stream, token, peer })
}

impl Receiver {
    pub async fn accept(&self) -> io::Result<Session> {
        let (mut stream, peer) = self.listener.accept().await?;
        let password = read_hello(&mut stream).await?;

        if let Some(expected) = &self.expected_password {
            let expected_bytes = expected.as_bytes();
            if password != expected_bytes {
                send_reject(&mut stream, "invalid password").await?;
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "invalid password",
                ));
            }
        }

        let token = generate_token();
        send_accept(&mut stream, &token).await?;
        Ok(Session { stream, token, peer })
    }
}

impl Session {
    #[allow(dead_code)]
    pub fn token(&self) -> &[u8] {
        &self.token
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer
    }

    pub async fn send(&mut self, payload: &[u8]) -> io::Result<()> {
        send_data(&mut self.stream, &self.token, payload).await
    }

    pub async fn recv(&mut self) -> io::Result<Vec<u8>> {
        read_data(&mut self.stream, &self.token).await
    }
}

async fn send_hello(stream: &mut TcpStream, password: &str) -> io::Result<()> {
    let bytes = password.as_bytes();
    if bytes.len() > MAX_PASSWORD_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "password too long",
        ));
    }

    write_header(stream, TYPE_HELLO).await?;
    write_u16(stream, bytes.len() as u16).await?;
    stream.write_all(bytes).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_hello(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let msg_type = read_header(stream).await?;
    if msg_type != TYPE_HELLO {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected HELLO",
        ));
    }

    let len = read_u16(stream).await? as usize;
    if len > MAX_PASSWORD_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "password too long",
        ));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn send_accept(stream: &mut TcpStream, token: &[u8]) -> io::Result<()> {
    if token.len() > MAX_TOKEN_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "token too long",
        ));
    }

    write_header(stream, TYPE_ACCEPT).await?;
    write_u16(stream, token.len() as u16).await?;
    stream.write_all(token).await?;
    stream.flush().await?;
    Ok(())
}

async fn send_reject(stream: &mut TcpStream, reason: &str) -> io::Result<()> {
    let bytes = reason.as_bytes();
    write_header(stream, TYPE_REJECT).await?;
    write_u16(stream, bytes.len() as u16).await?;
    stream.write_all(bytes).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_accept(stream: &mut TcpStream) -> io::Result<(Vec<u8>, SocketAddr)> {
    let msg_type = read_header(stream).await?;
    if msg_type == TYPE_REJECT {
        let reason = read_string(stream, MAX_PAYLOAD_LEN).await?;
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, reason));
    }
    if msg_type != TYPE_ACCEPT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected ACCEPT",
        ));
    }

    let len = read_u16(stream).await? as usize;
    if len > MAX_TOKEN_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "token too long",
        ));
    }
    let mut token = vec![0u8; len];
    stream.read_exact(&mut token).await?;
    let peer = stream.peer_addr()?;
    Ok((token, peer))
}

async fn send_data(stream: &mut TcpStream, token: &[u8], payload: &[u8]) -> io::Result<()> {
    if token.len() > MAX_TOKEN_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "token too long",
        ));
    }
    if payload.len() > MAX_PAYLOAD_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "payload too large",
        ));
    }

    write_header(stream, TYPE_DATA).await?;
    write_u16(stream, token.len() as u16).await?;
    stream.write_all(token).await?;
    write_u32(stream, payload.len() as u32).await?;
    stream.write_all(payload).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_data(stream: &mut TcpStream, expected_token: &[u8]) -> io::Result<Vec<u8>> {
    let msg_type = read_header(stream).await?;
    if msg_type != TYPE_DATA {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected DATA",
        ));
    }

    let token_len = read_u16(stream).await? as usize;
    if token_len > MAX_TOKEN_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "token too long",
        ));
    }

    let mut token = vec![0u8; token_len];
    stream.read_exact(&mut token).await?;
    if token != expected_token {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "invalid token",
        ));
    }

    let len = read_u32(stream).await? as usize;
    if len > MAX_PAYLOAD_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "payload too large",
        ));
    }
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload).await?;
    Ok(payload)
}

async fn write_header(stream: &mut TcpStream, msg_type: u8) -> io::Result<()> {
    stream.write_all(&MAGIC).await?;
    stream.write_all(&[VERSION, msg_type]).await?;
    Ok(())
}

async fn read_header(stream: &mut TcpStream) -> io::Result<u8> {
    let mut header = [0u8; 4];
    stream.read_exact(&mut header).await?;
    if header[0..2] != MAGIC {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid magic",
        ));
    }
    if header[2] != VERSION {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported version",
        ));
    }
    Ok(header[3])
}

async fn write_u16(stream: &mut TcpStream, value: u16) -> io::Result<()> {
    stream.write_all(&value.to_be_bytes()).await
}

async fn write_u32(stream: &mut TcpStream, value: u32) -> io::Result<()> {
    stream.write_all(&value.to_be_bytes()).await
}

async fn read_u16(stream: &mut TcpStream) -> io::Result<u16> {
    let mut buf = [0u8; 2];
    stream.read_exact(&mut buf).await?;
    Ok(u16::from_be_bytes(buf))
}

async fn read_u32(stream: &mut TcpStream) -> io::Result<u32> {
    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).await?;
    Ok(u32::from_be_bytes(buf))
}

async fn read_string(stream: &mut TcpStream, max_len: usize) -> io::Result<String> {
    let len = read_u16(stream).await? as usize;
    if len > max_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "string too long",
        ));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

fn generate_token() -> Vec<u8> {
    let mut token = vec![0u8; TOKEN_LEN];
    rand::rngs::OsRng.fill_bytes(&mut token);
    token
}
