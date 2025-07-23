//! Run:
//!
//! ```bash
//! cargo run --bin tcp-echo-server
//! ```
//!
//! Then connect to localhost:7000
//!
//! ```bash
//! nc localhost 7000
//! ```
//!
//! Anything you type will be echoed.

use anyhow::Context;
use libuv::prelude::*;
use libuv::{Buf, ReadonlyBuf, TcpBindFlags};
use std::net::Ipv4Addr;

const DEFAULT_PORT: u16 = 7000;
const DEFAULT_BACKLOG: i32 = 128;

fn main() -> anyhow::Result<()> {
    let mut l = Loop::default().context("new loop")?;

    let mut server = l.tcp().context("tcp")?;
    let addr = (Ipv4Addr::UNSPECIFIED, DEFAULT_PORT).into();
    server
        .bind(&addr, TcpBindFlags::empty())
        .map_err(|err| anyhow::anyhow!("bind: {err}"))?;
    server.listen(DEFAULT_BACKLOG, on_new_connection).context("listen")?;

    l.run(RunMode::Default).context("run")?;

    Ok(())
}

fn alloc_buffer(_: Handle, suggested_size: usize) -> Option<Buf> {
    Buf::with_capacity(suggested_size).ok()
}

fn echo_write(mut buf: ReadonlyBuf, status: libuv::Result<u32>) {
    if let Err(e) = status {
        eprintln!("Write error {e}");
    }
    buf.dealloc();
}

fn echo_read(mut client: StreamHandle, nread: libuv::Result<usize>, buf: ReadonlyBuf) {
    match nread {
        Ok(0) => {}
        Ok(_) => {
            if let Err(e) = client.write(&[buf], move |_, s| echo_write(buf, s)) {
                eprintln!("Error echoing to socket: {e}");
            }
        }
        Err(e) => {
            if e != libuv::Error::EOF {
                eprintln!("Read error {e}");
            }
            client.close(());
        }
    }
}

fn on_new_connection(mut server: StreamHandle, status: libuv::Result<u32>) {
    if let Err(e) = status {
        eprintln!("New connection error: {e}");
        return;
    }

    if let Ok(client) = server.get_loop().tcp().as_mut() {
        match server.accept(&mut client.to_stream()) {
            Ok(_) => {
                if let Err(e) = client.read_start(alloc_buffer, echo_read) {
                    eprintln!("Error starting read on client: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Error accepting connection: {e}");
                client.close(());
            }
        }
    }
}
