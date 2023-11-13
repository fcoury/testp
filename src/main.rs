use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{mpsc, Arc},
    thread,
};

use error::{Error, Result};

mod error;

struct Client {
    stream: Arc<TcpStream>,
}

struct Target {
    stream: Arc<TcpStream>,
}

struct Broadcaster {
    targets: Vec<Target>,
}

impl Broadcaster {
    fn new(targets: Vec<String>) -> Result<Self> {
        let connections: Result<Vec<_>> = targets
            .into_iter()
            .map(|target| {
                TcpStream::connect(target.clone())
                    .map_err(|cause| Error::ConnectionError { target, cause })
            })
            .collect();

        let targets = connections?
            .into_iter()
            .map(|stream| Target {
                stream: Arc::new(stream),
            })
            .collect();

        Ok(Self { targets })
    }

    fn new_broadcast(&mut self, bytes: &[u8]) -> Result<()> {
        for target in &self.targets {
            let stream = target.stream.clone();
            let bytes = bytes.to_vec();
            // TODO handle result below
            _ = stream.as_ref().write_all(&bytes);
        }

        Ok(())
    }
}

struct Server {
    clients: HashMap<SocketAddr, Client>,
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

fn client(stream: Arc<TcpStream>, tx: mpsc::Sender<Message>) -> Result<()> {
    let addr = stream.peer_addr()?;

    tx.send(Message::ClientConnected {
        stream: stream.clone(),
        addr,
    })?;

    let mut buffer = [0; 1024];
    loop {
        let n = stream.as_ref().read(&mut buffer)?;
        if n > 0 {
            let bytes: Box<[u8]> = buffer[..n].iter().cloned().collect();
            println!("Request: {}", String::from_utf8_lossy(&bytes));
            tx.send(Message::NewMessage { addr, bytes })?;
        } else {
            tx.send(Message::ClientDisconnected { addr })?;
            break;
        }
    }

    Ok(())
}

fn server(
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

fn broadcaster(
    targets: Vec<String>,
    receive_broadcast: mpsc::Receiver<Box<[u8]>>,
    send_message: mpsc::Sender<Message>,
) -> Result<()> {
    let mut broadcaster = Broadcaster::new(targets)?;

    // spawn the target threads (target -> server)
    for target in &broadcaster.targets {
        let stream = target.stream.clone();
        let send_message = send_message.clone();
        thread::spawn(|| target_client(stream, send_message));
    }

    loop {
        let bytes = receive_broadcast.recv()?;
        broadcaster.new_broadcast(&bytes)?;
    }
}

fn target_client(stream: Arc<TcpStream>, send_message: mpsc::Sender<Message>) -> Result<()> {
    let addr = stream.peer_addr()?;
    let mut buffer = [0; 1024];
    loop {
        let n = stream.as_ref().read(&mut buffer)?;
        if n > 0 {
            let bytes: Box<[u8]> = buffer[..n].iter().cloned().collect();
            println!(
                "Target {addr} incoming: {}",
                String::from_utf8_lossy(&bytes)
            );
            send_message.send(Message::BroadcastResponse { addr, bytes })?;
        } else {
            // TODO tx.send(Message::ClientDisconnected { addr })?;
            break;
        }
    }

    Ok(())
}

/// A sketch of the new architecture for the yprox multiplexing, modifying proxy server
///
/// The terminology I'm using here is:
///
/// Client - a client process that would connect to the application we're multiplexing
/// Server - the proxy server, a man-in-the-middle between the client and the application
/// Target - each server that acts as the application
///
fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:6000")?;
    let targets = vec!["127.0.0.1:6001".to_string(), "127.0.0.1:6002".to_string()];

    // used to send messages to the server
    let (send_message, receive_message) = mpsc::channel();

    // used to send broadcasts to all targets
    let (send_broadcast, receive_broadcast) = mpsc::channel();

    // spawn the server thread (handles server -> client and server -> broadcast)
    // handles messages between client and server, and sends broadcasts
    thread::spawn(|| server(receive_message, send_broadcast));

    // spawn the broadcasting thread (handles server -> targets and targets -> server)
    // the breadcaster receives broadcast requests and sends them to all targets
    // it also receives the send_message handle so that each target can send individual
    // responses to the server
    let send_message_clone = send_message.clone();
    thread::spawn(|| {
        broadcaster(targets, receive_broadcast, send_message_clone)
            .map_err(|err| eprintln!("{:?}", err))
    });

    // spawn the client threads (handle client -> server)
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let stream = Arc::new(stream);
                let send_message = send_message.clone();
                thread::spawn(|| client(stream, send_message));
            }
            Err(err) => {
                println!("Error accepting connection: {}", err);
            }
        }
    }

    Ok(())
}
