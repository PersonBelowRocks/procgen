extern crate bevy_ecs as ecs;
extern crate nalgebra as na;
extern crate volume as vol;

mod block;
mod chunk;
mod runtime;
mod util;

#[tokio::main]
async fn main() {
    env_logger::init();

    todo!()
}

#[cfg(test)]
mod tests {}
