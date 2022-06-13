extern crate downcast_rs as dc;
extern crate nalgebra as na;
extern crate thiserror as te;
extern crate volume as vol;

mod block;
mod chunk;
mod generation;
#[allow(dead_code)]
mod runtime;
mod util;

#[tokio::main]
async fn main() {
    env_logger::init();

    todo!()
}

#[cfg(test)]
mod tests {}
