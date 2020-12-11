use std::net::{SocketAddr, TcpStream};
use std::io::Write;

pub fn send(address: &SocketAddr, bytes: &Vec<u8>) -> std::io::Result<usize> {
    let written = TcpStream::connect(address)?.write(bytes)?;
    Ok(written)
}
