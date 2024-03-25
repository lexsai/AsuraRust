use std::net::{TcpListener, SocketAddr, TcpStream};
use std::thread::spawn;
use std::io;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

struct ProxyServer { 
    target_addr: SocketAddr
}

#[derive(Debug)]
struct Packet {
    packet_length: u32,
    packet_id: u8,
    packet_data_bytes: Vec<u8>,
}

impl Packet {
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes: Vec<u8> = Vec::new();
        bytes.extend(self.packet_length.to_le_bytes());
        bytes.push(self.packet_id);
        bytes.extend(&self.packet_data_bytes);
        bytes
    }
}

fn receive_packet(stream: &mut TcpStream) -> io::Result<Packet> {
    let mut packet_size_buffer = [0u8; 4];
    if let Err(error) = stream.read_exact(&mut packet_size_buffer) {
        stream.shutdown(std::net::Shutdown::Both)?;
        return Err(error);
    };
    let packet_length = u32::from_le_bytes(packet_size_buffer);

    let mut left_to_read = packet_length;

    let mut packet_id_buffer = [0u8; 1];
    stream.read_exact(&mut packet_id_buffer)?;
    let packet_id = packet_id_buffer[0];
    
    left_to_read -= 1;

    let mut packet_data_buffer = vec![0u8; left_to_read as usize];
    stream.read_exact(&mut packet_data_buffer)?;

    Ok(Packet {
        packet_length: packet_length,
        packet_id: packet_id,
        packet_data_bytes: packet_data_buffer
    })
}

fn pipe(incoming: &mut TcpStream, outgoing: &mut TcpStream, _proxy: &mut Arc<Mutex<ProxyServer>>) -> io::Result<()> {
    loop {
        let packet = receive_packet(incoming)?;

        if (&incoming.local_addr()?).to_string() == "127.0.0.1:6410" {
            println!("[CLIENT>PROXY] packet received with id {}", packet.packet_id);
        } else {
            println!("[SERVER>PROXY] packet received with id {}", packet.packet_id);
        }

        outgoing.write(&packet.to_bytes())?;
        outgoing.flush().unwrap();
    }
}

fn proxy_connection(mut incoming: TcpStream, proxy: &Arc<Mutex<ProxyServer>>) {
    println!("Client connected from: {:?}", incoming.peer_addr().unwrap());

    println!("Connecting to {:?}", proxy.lock().unwrap().target_addr);
    let mut outgoing = TcpStream::connect(proxy.lock().unwrap().target_addr).unwrap();

    let mut incoming_clone = incoming.try_clone().unwrap();
    let mut outgoing_clone = outgoing.try_clone().unwrap();

    let mut proxy_forward_clone = proxy.clone();
    let mut proxy_backward_clone = proxy.clone();

    let forward = spawn(move || pipe(&mut incoming, &mut outgoing, &mut proxy_forward_clone));
    let backward = spawn(move || pipe(&mut outgoing_clone, &mut incoming_clone, &mut proxy_backward_clone));

    println!("Proxying data...");

    let _ = forward.join().unwrap();
    let _ = backward.join().unwrap();

    println!("Socket closed");
}

fn main() {
    let proxy_server = Arc::new(Mutex::new(ProxyServer {
        target_addr: String::from("54.176.181.177:6410").parse::<SocketAddr>().unwrap()
    }));

    let listener = TcpListener::bind("127.0.0.1:6410").unwrap();
    
    for socket in listener.incoming() {
        let socket = socket.unwrap();

        println!("New proxy connection");
        let proxy_server_clone = proxy_server.clone();
        spawn(move || proxy_connection(socket, &proxy_server_clone));
    }

    println!("{}", proxy_server.lock().unwrap().target_addr);
}