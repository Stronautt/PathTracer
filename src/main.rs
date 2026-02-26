#![allow(dead_code)]

mod accel;
mod app;
mod camera;
mod gpu;
mod input;
mod io;
mod model;
mod picking;
mod render;
mod scene;
mod shaders;
mod ui;

use std::env;

use anyhow::Result;

fn main() -> Result<()> {
    env_logger::init();
    app::run(env::args().nth(1))
}
