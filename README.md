# simple_proxy
A simple async proxy server written in Rust.

This is a simple proxy server written in Rust.

# How to run it?

You can run the proxy in the following way:
./simple_proxy -l 4455 -r 192.168.1.110 -p 22

This will start the proxy listening at port 4455, and is a proxy to 192.168.1.110 at port 22.

The -l argument can be left out, the default listening port is 8080.
