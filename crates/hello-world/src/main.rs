use anyhow::Context;
use libuv::Loop;
use libuv::prelude::*;

fn main() -> anyhow::Result<()> {
    // Initialize the libuv loop
    let mut l = Loop::new().context("new loop")?;

    println!("Now quitting.");

    let _ = l.run(RunMode::Default);

    // Loop 的析构函数会调用 uv_loop_delete，底层会调用 close 函数。
    // 显示调用 close 函数会导致析构时断言失败。
    // l.close().context("close loop")?;

    Ok(())
}
