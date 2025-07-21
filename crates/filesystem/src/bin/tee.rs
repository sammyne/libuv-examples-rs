use anyhow::Context;
use libuv::prelude::*;
use libuv::{Buf, FsModeFlags, FsOpenFlags, PipeHandle, ReadonlyBuf};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).context("must pass a path to a file")?;

    let mut l = Loop::default()?;

    let mut stdin_pipe = l.pipe(false).context("new stdin pipe")?;
    stdin_pipe.open(STDIN).context("open stdin pipe")?;

    let mut stdout_pipe = l.pipe(false).context("new stdout pipe")?;
    stdout_pipe.open(STDOUT)?;

    let file = l
        .fs_open_sync(
            &path,
            FsOpenFlags::CREAT | FsOpenFlags::RDWR,
            FsModeFlags::OWNER_READ | FsModeFlags::OWNER_WRITE | FsModeFlags::GROUP_READ | FsModeFlags::OTHERS_READ,
        )
        .map_err(|err| anyhow::anyhow!("open file: {err}"))?;
    let mut file_pipe = l.pipe(false).context("new file pipe")?;
    file_pipe.open(file).context("open file pipe")?;

    stdin_pipe
        .read_start(alloc_buffer, move |stream, nread, buf| {
            read_stdin(stream, &mut stdout_pipe, &mut file_pipe, nread, buf)
        })
        .context("pipe stdin read")?;

    l.run(RunMode::Default).context("run loop")?;

    // We need to close the pipes...
    stdin_pipe.close(());
    stdout_pipe.close(());
    file_pipe.close(());

    // Restart the loop just to close the pipes... this should return fairly quickly.
    l.run(RunMode::Default).context("run loop again")?;

    Ok(())
}

const STDIN: libuv::File = 0;
const STDOUT: libuv::File = 1;

fn alloc_buffer(_handle: Handle, suggested_size: usize) -> Option<Buf> {
    match Buf::with_capacity(suggested_size) {
        Ok(b) => Some(b),
        Err(e) => {
            eprintln!("error allocating buffer: {e}");
            None
        }
    }
}

fn read_stdin(
    mut stdin_pipe: StreamHandle,
    stdout_pipe: &mut PipeHandle,
    file_pipe: &mut PipeHandle,
    nread: libuv::Result<usize>,
    mut buf: ReadonlyBuf,
) {
    match nread {
        Err(e) => {
            if e != libuv::Error::EOF {
                eprintln!("error reading stdin: {e}");
            }

            // The original example closed all of the pipe here. However, since the writes are
            // asynchronous, the close could potentially happen before the write callback is fired.
            // If this happens, the callback will report that the write has been ECANCELED even
            // though it succeeded. The better way to handle this is to stop the read here, which
            // will cause the loop to exit after all of the writes have finished. *Then* we can
            // close the pipes.
            if let Err(e) = stdin_pipe.read_stop() {
                eprintln!("error stopping the read: {e}");
            }
        }
        Ok(len) => {
            if len > 0 {
                if let Err(e) = write_data(stdout_pipe.to_stream(), len as _, &buf) {
                    eprintln!("error preparing to writing to stdout: {e}");
                }

                if let Err(e) = write_data(file_pipe.to_stream(), len as _, &buf) {
                    eprintln!("error preparing to writing to file: {e}");
                }
            }
        }
    }

    // free memory in the ReadonlyBuf
    buf.dealloc();
}

fn write_data(mut stream: StreamHandle, len: usize, buf: &ReadonlyBuf) -> libuv::Result<()> {
    let mut buf = Buf::new_from(buf, Some(len))?;
    stream.write(&[buf], move |_, status| {
        if let Err(e) = status {
            eprintln!("error writing data: {e}");
        }
        buf.destroy();
    })?;
    Ok(())
}
