pub mod packets;
use log::{error, info};
use packets::*;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub mod types;

use std::{
    error::Error,
    io::{Read, Write},
    sync::mpsc::{Receiver, SendError, Sender},
};

use self::types::*;

pub const PROTOCOL_1_17_1: VarInt = VarInt(756);
pub const PROTOCOL_1_18_1: VarInt = VarInt(757);

pub enum ServerState {
    Status,
    Login,
    Play,
}

/// Attempts to get the next packet in the TcpStream
/// Panics if the TcpStream could not read the next data to prevent correupted packets and unexpected behaviour
/// # Returns
///
/// Returns a Decoded Packet ready for processing, or None if there was no packet to receive.
///
async fn next_packet(stream: &mut TcpStream, state: ServerState) -> Option<DecodedPacket> {
    // Check there is packet and get size of it
    match VarInt::from_stream(stream).await {
        Ok(Some(VarInt(0))) => {
            return None;
        }
        Ok(Some(VarInt(len))) => {
            let mut buf = vec![0; len as usize];

            match stream.read_exact(&mut buf).await {
                Ok(_) => {
                    // Return packet without decompressing
                    return Some(decode_packet(buf, &state));
                }
                Err(e) => {
                    error!("Failed reading packet from stream: {}", e);
                    panic!("Force stopped to prevent unexpected behaviour.");
                }
            }
        }
        Ok(None) => {
            error!("Failed to read packet!");
            return None;
        }
        Err(_) => {
            return None;
        }
    }
}

pub async fn status(
    stream: &mut TcpStream,
) -> Result<StatusResponse, Box<dyn Error + Send + Sync>> {
    use std::net::SocketAddr;

    // Extracts local address from TcpStream
    let local_addr = match stream.local_addr() {
        Err(e) => {
            error!("Failed to get local address from TCPStream!");
            return Err(Box::new(e));
        }
        Ok(addr) => match addr {
            SocketAddr::V4(local) => local.ip().to_string(),
            SocketAddr::V6(local) => local.ip().to_string(),
        },
    };

    // Construct and send handshake and login packets
    let handshake = DecodedPacket::Handshake(Handshake {
        protocol_version: VarInt(-1),
        origin: MCString(local_addr),
        port: Short(0),
        next_state: HandshakeMode::Status,
    });

    send_packet(stream, handshake).await?;
    info!("Sent handshake");
    send_packet(stream, DecodedPacket::StatusRequest(StatusRequest {})).await?;
    info!("Sent status request");
    send_packet(stream, DecodedPacket::StatusPing(StatusPing {})).await?;
    info!("Sent ping");

    loop {
        match next_packet(stream, ServerState::Status).await {
            Some(DecodedPacket::StatusResponse(response)) => {
                return Ok(response);
            }
            None => {
                info!("No response");
            }
            Some(DecodedPacket::StatusPong(payload)) => {
                info!("Got pong");
            }
            _ => {
                break;
            }
        }
    }

    todo!();
    // TODO - Finish this.
}

/// Sends a packet to the server
///
/// # Returns
///
/// * `Some(())` if the packet is successfully sent
/// * `None` if it is not
async fn send_packet(
    stream: &mut TcpStream,
    packet: DecodedPacket,
) -> Result<bool, Box<dyn Error + Send + Sync>> {
    // Attempt to encode packet
    match packet.encode() {
        Some(pack) => {
            // Send without compression
            let bytes = pack.get_bytes_with_length();
            match stream.write(bytes.as_slice()).await {
                Ok(_) => Ok(true),
                Err(e) => {
                    error!("Failed to write to TcpStream: {}", e);
                    return Err(Box::new(e));
                }
            }
        }
        // Packet encode failure
        None => {
            error!("Failed to encode packet: {:?}", packet);
            return Ok(false);
        }
    }
}

// Struct to hold communication channels between network manager and other threads
pub struct NetworkChannel {
    pub send: Sender<NetworkCommand>,
    pub recv: Receiver<NetworkCommand>,
}

impl NetworkChannel {
    pub fn send_packet(&self, packet: DecodedPacket) -> Result<(), SendError<NetworkCommand>> {
        match self.send.send(NetworkCommand::SendPacket(packet)) {
            Ok(_) => Ok(()),
            Err(e) => {
                error!("Failed to communicate with network commander: {:?}", e);
                Err(e)
            }
        }
    }
}

// Types of Messages that can be sent
#[derive(Debug)]
pub enum NetworkCommand {
    Ok,
    Error(Box<dyn Error + Send + Sync>),
    Disconnect,
    // Login(protocol, port, name)
    Login(VarInt, Short, MCString),
    Status,

    SendPacket(DecodedPacket),
    ReceivePacket(DecodedPacket),
}
