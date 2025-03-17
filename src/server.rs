use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use crate::{
    ProxyArgs, id_gen::IncrementalIdGeneratorAtomic, quinn_example_code::make_server_endpoint,
};
use byteorder::{ByteOrder, LittleEndian};
use bytes::Bytes;
use futures::lock::Mutex;
use quinn::Connection;
use std::error::Error;
use tokio::net::UdpSocket;

pub struct MapsServer {
    id_to_addr: HashMap<u16, SocketAddr>,
    addr_to_id: HashMap<SocketAddr, u16>,
    id_gen: IncrementalIdGeneratorAtomic,
}

impl MapsServer {
    pub fn new() -> Self {
        Self {
            addr_to_id: HashMap::new(),
            id_to_addr: HashMap::new(),
            id_gen: IncrementalIdGeneratorAtomic::new(),
        }
    }
    pub fn get_id(&mut self, addr: &SocketAddr) -> u16 {
        match self.addr_to_id.get(addr) {
            None => {
                let id = self.id_gen.next() as u16;
                self.addr_to_id.insert(*addr, id);
                self.id_to_addr.insert(id, *addr);
                id
            }
            Some(id) => *id,
        }
    }
    pub fn get_addr(&self, id: u16) -> Option<SocketAddr> {
        self.id_to_addr.get(&id).cloned()
    }
}

pub async fn run_proxy(
    args: ProxyArgs,
) -> anyhow::Result<(), Box<dyn Error + Send + Sync + 'static>> {
    println!(
        " >>> starting UDP proxy on | clients ::{} | server ::{}",
        args.port_client, args.port_server
    );

    let quic_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), args.port_server.parse()?);
    let udp_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), args.port_client.parse()?);

    let maps: Arc<Mutex<MapsServer>> = Arc::new(Mutex::new(MapsServer::new()));

    loop {
        let (endpoint, _server_cert) = make_server_endpoint(quic_addr).unwrap();
        // accept a single connection
        let incoming_conn = endpoint.accept().await.unwrap();
        let quic_connection = incoming_conn.await.unwrap();
        println!(
            "[server] proxy connection accepted: addr={}",
            quic_connection.remote_address()
        );

        let udp_socket = UdpSocket::bind(udp_addr).await?;

        let future_quic = quic_server(&quic_connection, &udp_socket, maps.clone());
        let future_udp = udp_server(&quic_connection, &udp_socket, maps.clone());
        let result = futures::try_join!(future_quic, future_udp);

        let reason = match result {
            Ok(_) => "Ok".into(),
            Err(e) => format!("Error: {}", e),
        };
        println!("Connection closed, restarting. Reason: {}", reason);
    }

    Ok(())
}

pub async fn udp_server(
    quic_conn: &Connection,
    udp_socket: &UdpSocket,
    maps: Arc<Mutex<MapsServer>>,
) -> anyhow::Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let mut buf = [0u8; 65535];

    loop {
        let (len, addr) = udp_socket.recv_from(&mut buf[2..]).await?;

        let id = {
            let mut lock = maps.lock().await;
            lock.get_id(&addr)
        };
        LittleEndian::write_u16(&mut buf[0..2], id);
        let bytes = Bytes::copy_from_slice(&buf[0..len + 2]);
        quic_conn.send_datagram(bytes)?;
        
        println!("[server] UDP packet received: id={:?} data={:?}", id, &buf[2..len+2]);
    }
}

pub async fn quic_server(
    quic_conn: &Connection,
    udp_socket: &UdpSocket,
    maps: Arc<Mutex<MapsServer>>,
) -> anyhow::Result<(), Box<dyn Error + Send + Sync + 'static>> {
    loop {
        let datagram = quic_conn.read_datagram().await?;
        let id = LittleEndian::read_u16(&datagram[0..2]);

        let addr = {
            let lock = maps.lock().await;
            lock.get_addr(id)
        };

        if let Some(addr) = addr {
            udp_socket.send_to(&datagram[2..], addr).await?;
        }
        println!("[server] QUIC datagram received: id={:?} data={:?}", id, &datagram[2..]);
    }
}
