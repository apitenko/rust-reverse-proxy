#![feature(addr_parse_ascii)]
#![feature(try_blocks)]

use std::{error::Error, time::Duration};

use clap::{Args, Parser, Subcommand};
use client::run_connect;
use server::run_proxy;
use tokio::time::sleep;

pub mod client;
pub mod id_gen;
pub mod quinn_example_code;
pub mod server;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Proxy(ProxyArgs),
    Connect(ConnectArgs),
}

#[derive(Args)]
pub struct ProxyArgs {
    #[arg(
        short = 'c',
        long,
        help = "Listen port for the clients (all interfaces, UDP)"
    )]
    port_client: String,
    #[arg(
        short = 's',
        long,
        help = "This proxy listen port (all interfaces, QUIC)"
    )]
    port_server: String,
}

#[derive(Args)]
pub struct ConnectArgs {
    #[arg(short = 'r', long, help = "Local receiver port (localhost, UDP)")]
    port_receiver: String,
    #[arg(short = 'a', long, help = "Remote proxy address (QUIC)")]
    remote_address: String,
    #[arg(short = 'p', long, help = "Remote proxy port (QUIC)")]
    remote_port: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let cli = Cli::parse();

    loop {
        let result = match &cli.command {
            Commands::Proxy(args) => run_proxy(args).await,
            Commands::Connect(args) => run_connect(args).await,
        };

        result.unwrap();

        // match result {
        //     Err(e) => println!("Fatal error: {e}"),
        //     Ok(_) => (),
        // }
        sleep(Duration::from_secs(1)).await;
    }
    Ok(())
}
