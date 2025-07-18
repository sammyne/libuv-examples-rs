use anyhow::Context;
use libuv::{IdleHandle, Loop, RunMode};

fn main() -> anyhow::Result<()> {
    let mut l = Loop::default().context("new loop")?;

    let mut idle = l.idle().context("new idle")?;

    let mut c = 0i64;

    let callback = move |mut h: IdleHandle| {
        c += 1;

        if c >= 5 {
            h.stop().expect("stop idle");
        }
    };

    idle.start(callback).context("start idle")?;

    println!("Idling...");
    l.run(RunMode::Default).context("run loop")?;

    Ok(())
}
