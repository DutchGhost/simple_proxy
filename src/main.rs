extern crate clap;
extern crate tokio;

use clap::{App, Arg};

use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

use std::net::SocketAddr;
use std::mem::drop;

fn main() {
    let (server_addr, host_addr) = parse_args();

    let tcp = TcpListener::bind(&server_addr).unwrap();

    // Iterate incoming connections
    let server = tcp
        .incoming()
        .for_each(move |tcp| {
            let proxy = proxy(tcp, host_addr).map(drop).map_err(|e| {
                println!("err {}", e);
            });

            tokio::spawn(proxy);

            Ok(())
        })
        .map_err(|err| {
            println!("server error {:?}", err);
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
        })
        .map_err(|err| {
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

        let proxy = sending.select(receiving).map(drop).map_err(|(e, _)| {
            println!("Error: {:?}", e);
        });

        tokio::spawn(proxy);
        Ok(())
    })
}

/// Parses commandline arguments into 2 socket addresses.
/// The first socket address is the server itself, the second is the host this server is a proxy to.
fn parse_args() -> (SocketAddr, SocketAddr) {
    let matches = App::new("SimpleProxy")
        .version("0.0.1")
        .author("DutchGhost")
        .about("Proxy between 2 connections")
        .arg(
            Arg::with_name("localport")
                .short("l")
                .long("localport")
                .value_name("STRING")
                .help("Sets the port the proxy should listen on. Defaults to 8080.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("remotehost")
                .short("r")
                .long("remotehost")
                .value_name("STRING")
                .help("The host to connect to")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name("remoteport")
                .short("p")
                .long("remoteport")
                .value_name("STRING")
                .help("The port of the host to connect to")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let localport = matches.value_of("localport").unwrap_or_else(|| "8080");
    let remotehost = matches.value_of("remotehost").unwrap();
    let remoteport = matches.value_of("remoteport").unwrap();

    let server = format!("0.0.0.0:{}", localport).parse().unwrap();

    let host = format!("{}:{}", remotehost, remoteport).parse().unwrap();

    (server, host)
}
