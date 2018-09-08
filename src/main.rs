extern crate structopt;
extern crate tokio;

use structopt::StructOpt;

use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

use std::mem::drop;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(
        short = "l",
        long = "localport",
        parse(try_from_str),
        default_value = "8080"
    )]
    localport: u16,

    #[structopt(short = "r", long = "remotehost", parse(try_from_str))]
    remotehost: IpAddr,

    #[structopt(short = "p", long = "remoteport", parse(try_from_str))]
    remoteport: u16,
}

fn main() {
    let app = Cli::from_args();

    let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), app.localport);
    let host_addr = SocketAddr::new(app.remotehost, app.remoteport);

    let tcp = TcpListener::bind(&server_addr).unwrap();

    // Iterate incoming connections
    let server = tcp
        .incoming()
        .for_each(move |tcp| {
            let proxy = proxy(tcp, host_addr).map(drop).map_err(|err| {
                // <-- error from proxy to host
                eprintln!("err {}", err);
            });

            tokio::spawn(proxy);

            Ok(())
        }).map_err(|err| {
            // <-- error from incomming connections
            eprintln!("server error {:?}", err);
        });

    // Start the runtime and spin up the server
    tokio::run(server);
}

/// A wrapper function over io::copy. Transform's the Future returned by io::copy into a Future<Item= (), Error = ()>
fn handled_copy<T, T2, T3, E, F>(future: F) -> impl Future<Item = (), Error = ()>
where
    T: std::fmt::Display,
    E: std::fmt::Debug,
    F: Future<Item = (T, T2, T3), Error = E>,
{
    future
        .map(|(n, _, _)| {
            println!("wrote {} bytes", n);
        }).map_err(|err| {
            // <-- io::copy error
            eprintln!("IO Error: {:?}", err);
        })
}

/// Takes in an object that implements both AsyncRead and AsyncWrite, representing the proxy-server.
/// As a second parameter, it takes the SocketAddr of the host this proxy should point to.
///
/// It performs an io::copy from the client to the host,
/// and an io::copy from the host to the client.
fn proxy<R>(server: R, host: SocketAddr) -> impl Future<Item = (), Error = io::Error>
where
    R: AsyncRead + AsyncWrite + Send + Sync + 'static,
{
    let (server_reader, server_writer) = server.split();

    TcpStream::connect(&host).and_then(move |stream| {
        let (host_reader, host_writer) = stream.split();

        let sending = handled_copy(io::copy(server_reader, host_writer));
        let receiving = handled_copy(io::copy(host_reader, server_writer));

        let proxy = sending.select(receiving).map(drop).map_err(|(err, _)| {
            eprintln!("Error: {:?}", err);
        });

        tokio::spawn(proxy);
        Ok(())
    })
}
