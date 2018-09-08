extern crate structopt;
extern crate tokio;

use structopt::StructOpt;

use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

use std::mem::drop;
use std::net::{IpAddr, SocketAddr};

#[derive(Debug, StructOpt)]
#[structopt(name = "SimpleProxy", about = "A simple, async proxy")]
struct Cli {
    /// Localhost, can be set to internal only.
    #[structopt(
        long = "localhost",
        parse(try_from_str),
        default_value = "0.0.0.0"
    )]
    localhost: IpAddr,

    /// The port the proxy should run on.
    #[structopt(short = "l", parse(try_from_str), default_value = "8080")]
    localport: u16,

    /// The host this server is a proxy to.
    #[structopt(short = "r", long = "remotehost", parse(try_from_str))]
    remotehost: IpAddr,

    /// The port of the host this server should proxy to.
    #[structopt(short = "p", long = "remoteport", parse(try_from_str))]
    remoteport: u16,
}

fn main() {
    let app = Cli::from_args();

    let server_addr = SocketAddr::new(app.localhost, app.localport);
    let host_addr = SocketAddr::new(app.remotehost, app.remoteport);

    let tcp = TcpListener::bind(&server_addr).unwrap();

    let server = tcp
        .incoming()
        .for_each(move |tcp| {
            tokio::spawn(proxy(tcp, host_addr));

            Ok(())
        }).map_err(|err| {
            eprintln!("An error occured while accepting connections: {}", err);
        });

    // Start the runtime and spin up the server
    tokio::run(server);
}

/// A wrapper function over io::copy. Transform's the Future returned by io::copy into a Future<Item= (), Error = io::Error>
fn copy<R, W>(reader: R, writer: W) -> impl Future<Item = (), Error = io::Error>
where
    R: AsyncRead,
    W: AsyncWrite,
{
    io::copy(reader, writer).map(|(n, ..)| println!("wrote {} bytes", n))
}

/// Takes in an object that implements both AsyncRead and AsyncWrite, representing the proxy-server.
/// As a second parameter, it takes the SocketAddr of the host this proxy should point to.
///
/// It performs an io::copy from the client to the host,
/// and an io::copy from the host to the client.
fn proxy<R>(server: R, host: SocketAddr) -> impl Future<Item = (), Error = ()>
where
    R: AsyncRead + AsyncWrite + Send + Sync + 'static,
{
    let (server_reader, server_writer) = server.split();

    TcpStream::connect(&host)
        .and_then(move |stream| {
            let (host_reader, host_writer) = stream.split();

            let sending = copy(server_reader, host_writer);
            let receiving = copy(host_reader, server_writer);

            let proxy = sending.select(receiving).map(drop).map_err(|(err, _)| {
                eprintln!("An error occured while servering the proxy: {:?}", err);
            });

            tokio::spawn(proxy);
            Ok(())
        }).map_err(|err| {
            eprintln!("Could not connect to host: {}", err);
        })
}
