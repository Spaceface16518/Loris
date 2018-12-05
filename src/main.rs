#![allow(dead_code)]
extern crate futures;
extern crate rand;
extern crate tokio;

use futures::{Async, Future, Poll, Stream};
use tokio::{
    io,
    net::{ TcpStream},
    timer::Interval,
};
use tokio::io::Write;

use std::time::{Duration, Instant};

const DELAY: u64 = 5000;

fn main() {
    let addr = "127.0.0.1:8080".parse().unwrap();
    let task = TcpStream::connect(&addr)
        .and_then(move |mut stream| {
            println!("created stream");
            stream.write(b"GET / HTTP/1.1\n");
            Interval::new(Instant::now(), Duration::from_millis(DELAY))
                .for_each(|_| {
                    io::write_all(stream, b"keep-alive\n").then(|result| {
                        println!("wrote keep-alive packet; success: {:?}", result.is_ok());
                        stream
                    })
                }).map_err(|e| panic!("Interval errored: {:?}", e))
        })
        .map_err(|err| {
            eprintln!("connection error = {:?}", err);
        });

    tokio::run(task);
}
