use anyhow::Context;
use libuv::prelude::*;
use libuv::{Buf, FsModeFlags, FsOpenFlags, FsReq, Loop};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).context("miss file")?;

    // Initialize the libuv loop
    let mut l = Loop::new().context("new loop")?;

    l.fs_open(&path, FsOpenFlags::RDONLY, FsModeFlags::empty(), on_open)
        .map_err(|err| anyhow::anyhow!("open file: {err}"))?;

    let _ = l.run(RunMode::Default);

    // Loop 的析构函数会调用 uv_loop_delete，底层会调用 close 函数。
    // 显示调用 close 函数会导致析构时断言失败。
    // l.close().context("close loop")?;

    Ok(())
}

const STDOUT: libuv::File = 1;

fn on_open(req: FsReq) {
    match req.result() {
        Ok(fd) => {
            let mut buf = match Buf::with_capacity(1024) {
                Ok(buf) => buf,
                Err(e) => {
                    eprintln!("error allocating a buffer: {}", e);
                    return;
                }
            };

            let fd = fd as libuv::File;
            if let Err(e) = req.r#loop().fs_read(fd, &[buf], -1, move |req| on_read(fd, req, buf)) {
                eprintln!("error starting read: {}", e);
                buf.destroy();
            }
        }
        Err(e) => eprintln!("error opening file: {}", e),
    }
}

fn on_read(fd: libuv::File, req: FsReq, mut buf: Buf) {
    match req.result() {
        Err(e) => {
            eprintln!("Read error: {e}");
            buf.destroy();
        }
        Ok(0) => {
            buf.destroy();
            if let Err(e) = req.r#loop().fs_close_sync(fd) {
                eprintln!("error closing file: {e}");
            }
        }
        Ok(len) => {
            if let Err(e) = buf.resize(len as _) {
                eprintln!("error resizing buffer: {e}");
                buf.destroy();
            }
            if let Err(e) = req
                .r#loop()
                .fs_write(STDOUT, &[buf], -1, move |req| on_write(req, fd, buf))
            {
                eprintln!("error starting write: {e}");
                buf.destroy();
            }
        }
    }
}

fn on_write(req: FsReq, file: libuv::File, mut buf: Buf) {
    if let Err(e) = req.result() {
        eprintln!("Write error: {e}");
        buf.destroy();
    } else if let Err(e) = req
        .r#loop()
        .fs_read(file, &[buf], -1, move |req| on_read(file, req, buf))
    {
        eprintln!("error continuing read: {e}");
    }
}
