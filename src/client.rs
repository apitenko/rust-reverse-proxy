use std::{
    collections::HashMap,
    error::Error,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs},
    sync::Arc,
};

use byteorder::{ByteOrder, LittleEndian};
use bytes::Bytes;
use futures::lock::Mutex;
use quinn::{ClientConfig, Connection, Endpoint, crypto::rustls::QuicClientConfig, rustls};
use tokio::net::UdpSocket;

use crate::{ConnectArgs, quinn_example_code::SkipServerVerification};

pub struct MapsClient {
    id_to_socket: HashMap<u16, Arc<UdpSocket>>,
}

impl MapsClient {
    pub fn new() -> Self {
        Self {
            id_to_socket: HashMap::new(),
        }
    }

    pub fn set_socket(&mut self, id: u16, socket: UdpSocket) {
        self.id_to_socket.insert(id, Arc::new(socket));
    }
    pub fn get_socket(&self, id: u16) -> Option<&Arc<UdpSocket>> {
        self.id_to_socket.get(&id)
    }
}

pub async fn run_connect(
    args: ConnectArgs,
) -> anyhow::Result<(), Box<dyn Error + Send + Sync + 'static>> {
    println!(
        " >>> connecting to UDP proxy on {}:{} forwarding to localhost:{}",
        args.remote_address, args.remote_port, args.port_receiver
    );

    let remote_addr = format!("{}:{}", args.remote_address, args.remote_port);
    let remote_addr = remote_addr.to_socket_addrs()?.next().unwrap();

    let mut client_endpoint = if remote_addr.is_ipv4() {
        Endpoint::client(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0))?
    } else {
        Endpoint::client(SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0))?
    };

    client_endpoint.set_default_client_config(ClientConfig::new(Arc::new(
        QuicClientConfig::try_from(
            rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(SkipServerVerification::new())
                .with_no_client_auth(),
        )?,
    )));

    // connect to server
    let quic_connection = client_endpoint
        .connect(remote_addr, "proxy")
        .unwrap()
        .await
        .unwrap();
    println!(
        "[client] connected: addr={}",
        quic_connection.remote_address()
    );

    let udp_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), args.port_receiver.parse()?);
    let maps: Arc<Mutex<MapsClient>> = Arc::new(Mutex::new(MapsClient::new()));

    loop {
        quic_client(&quic_connection, udp_address.clone(), maps.clone()).await?;
    }

    // Dropping handles allows the corresponding objects to automatically shut down
    drop(quic_connection);
    // Make sure the server has a chance to clean up
    client_endpoint.wait_idle().await;

    Ok(())
}

pub async fn quic_client(
    quic_conn: &Connection,
    udp_address: SocketAddr,
    maps: Arc<Mutex<MapsClient>>,
) -> anyhow::Result<(), Box<dyn Error + Send + Sync + 'static>> {
    loop {
        let datagram = quic_conn.read_datagram().await?;

        let id = LittleEndian::read_u16(&datagram[0..2]);

        {
            let mut lock = maps.lock().await;
            let socket = match lock.get_socket(id) {
                Some(socket) => socket,
                None => {
                    let udp_endpoint = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
                    let udp_socket = UdpSocket::bind(udp_endpoint).await?;
                    udp_socket.connect(udp_address).await?;
                    lock.set_socket(id, udp_socket);
                    let socket = lock.get_socket(id).unwrap();
                    tokio::spawn(run_socket_client(id, quic_conn.clone(), socket.clone()));
                    socket
                }
            };
            println!(
                "[client] QUIC datagram received: id={:?} data={:?}",
                id,
                &datagram[2..]
            );
            socket.send(&datagram[2..]).await?;
        }
    }
}

pub async fn run_socket_client(
    id: u16,
    quic_conn: Connection,
    socket: Arc<UdpSocket>,
) -> anyhow::Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let mut buf = [0u8; 65535];
    LittleEndian::write_u16(&mut buf[0..2], id);
    loop {
        let len = socket.recv(&mut buf[2..]).await?;
        quic_conn.send_datagram(Bytes::copy_from_slice(&buf[..len + 2]))?;
        println!(
            "[client] UDP packet received: id={:?} data={:?}",
            id,
            &buf[2..len + 2]
        );
    }
}
