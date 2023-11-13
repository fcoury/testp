use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{mpsc, Arc},
    thread,
};

use broadcaster::broadcaster;
use client::client;
use error::{Error, Result};
use server::server;
use target::Target;

mod broadcaster;
mod client;
mod error;
mod server;
mod target;

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
