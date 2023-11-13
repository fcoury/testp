use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{mpsc, Arc},
    thread,
};

use crate::{Error, Message, Result, Target};

pub fn broadcaster(
    targets: Vec<SocketAddr>,
    receive_broadcast: mpsc::Receiver<Box<[u8]>>,
    send_message: mpsc::Sender<Message>,
) -> Result<()> {
    let mut broadcaster = Broadcaster::new(targets)?;

    // spawn the target threads (target -> server)
    for t in &broadcaster.targets {
        let stream = t.stream.clone();
        let send_message = send_message.clone();
        thread::spawn(|| target(stream, send_message));
    }

    loop {
        let bytes = receive_broadcast.recv()?;
        broadcaster.new_broadcast(&bytes)?;
    }
}

fn target(stream: Arc<TcpStream>, send_message: mpsc::Sender<Message>) -> Result<()> {
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
            send_message.send(Message::NewTargetMessage { addr, bytes })?;
        } else {
            send_message.send(Message::TargetDisconnected { addr })?;
            break;
        }
    }

    Ok(())
}

struct Broadcaster {
    targets: Vec<Target>,
}

impl Broadcaster {
    fn new(targets: Vec<SocketAddr>) -> Result<Self> {
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
