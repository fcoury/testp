use std::{
    collections::HashMap,
    io::Write,
    net::{SocketAddr, TcpStream},
    sync::{mpsc, Arc},
};

use crate::{client::Client, Result};

pub fn server(
    receive_message: mpsc::Receiver<Message>,
    send_broadcast: mpsc::Sender<Box<[u8]>>,
) -> Result<()> {
    let mut server = Server::new();

    for message in receive_message {
        match message {
            Message::ClientConnected { stream, addr } => {
                server.client_connected(stream, addr);
            }
            Message::ClientDisconnected { addr } => {
                server.client_disconnected(addr);
            }
            Message::TargetConnected {
                stream: _stream,
                addr,
            } => {
                server.target_connected(addr);
            }
            Message::TargetDisconnected { addr } => {
                server.target_disconnected(addr);
            }
            Message::NewMessage { addr, bytes } => {
                server.new_message(addr, &bytes);
                send_broadcast.send(bytes)?;
            }
            Message::BroadcastResponse { addr, bytes } => {
                server.new_response(addr, &bytes);
            }
        }
    }

    Ok(())
}

pub struct Server {
    pub clients: HashMap<SocketAddr, Client>,
}

impl Server {
    fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    fn client_connected(&mut self, stream: Arc<TcpStream>, addr: SocketAddr) {
        println!("Client connected: {}", addr);
        self.clients.insert(addr, Client { stream });
    }

    fn client_disconnected(&mut self, addr: SocketAddr) {
        println!("Client disconnected: {}", addr);
        self.clients.remove(&addr);
    }

    fn target_connected(&mut self, addr: SocketAddr) {
        println!("Target connected: {}", addr);
    }

    fn target_disconnected(&mut self, addr: SocketAddr) {
        println!("Target disconnected: {}", addr);
    }

    fn new_message(&mut self, addr: SocketAddr, bytes: &[u8]) {
        println!("New message from {}: {:?}", addr, bytes);
    }

    fn new_response(&mut self, addr: SocketAddr, bytes: &[u8]) {
        println!("New response from {}: {:?}", addr, bytes);
        for client in self.clients.values() {
            // TODO: handle result below
            _ = client.stream.as_ref().write_all(bytes);
        }
    }
}

pub enum Message {
    ClientConnected {
        stream: Arc<TcpStream>,
        addr: SocketAddr,
    },
    ClientDisconnected {
        addr: SocketAddr,
    },
    TargetConnected {
        stream: Arc<TcpStream>,
        addr: SocketAddr,
    },
    TargetDisconnected {
        addr: SocketAddr,
    },
    NewMessage {
        addr: SocketAddr,
        bytes: Box<[u8]>,
    },
    BroadcastResponse {
        addr: SocketAddr,
        bytes: Box<[u8]>,
    },
}
