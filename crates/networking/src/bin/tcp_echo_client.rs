use anyhow::Context;
use libuv::{Buf, ReadonlyBuf};
use libuv::{ConnectReq, Result, WriteReq, prelude::*};
use std::net::Ipv4Addr;

const DEFAULT_PORT: u16 = 7000;

fn main() -> anyhow::Result<()> {
    let mut l = Loop::default().context("new loop")?;

    let mut c = l.tcp().context("tcp")?;
    let addr = (Ipv4Addr::UNSPECIFIED, DEFAULT_PORT).into();

    c.connect(&addr, on_connect)
        .map_err(|err| anyhow::anyhow!("connect: {err}"))?;

    l.run(RunMode::Default).context("run")?;

    Ok(())
}

fn alloc_buffer(_: Handle, suggested_size: usize) -> Option<Buf> {
    Buf::with_capacity(suggested_size).ok()
}

fn on_connect(req: ConnectReq, status: Result<u32>) {
    if let Err(e) = status {
        eprintln!("Connect error: {e}");
        return;
    }

    let mut conn = req.handle();

    let buf = Buf::new("hello world").expect("new buffer");
    conn.write(&[buf], on_write).expect("write buf");
}

fn on_write(req: WriteReq, status: libuv::Result<u32>) {
    if let Err(e) = status {
        eprintln!("Write error: {e}");
    }

    let mut conn = req.handle();

    if let Err(e) = conn.read_start(alloc_buffer, echo_read) {
        eprintln!("Error starting read on client: {}", e);
    }
}

fn echo_read(mut conn: StreamHandle, nread: libuv::Result<usize>, buf: ReadonlyBuf) {
    match nread {
        Ok(0) => {}
        Ok(_) => {
            let reply = buf.to_string_lossy().expect("stringify reply");   
            println!("reply = {reply}");
        }
        Err(e) => {
            if e != libuv::Error::EOF {
                eprintln!("Read error {e}");
            }
            conn.close(());
        }
    }
}