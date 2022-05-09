extern crate nalgebra as na;

mod block;
mod chunk;
mod generate;
mod net;
mod util;
mod volume;

#[tokio::main]
async fn main() {
    env_logger::init();

    let _server = net::server::Server::new()
        .with_version(net::protocol::ProtocolVersion::V1)
        .with_compression_threshold(256);
}

#[cfg(test)]
mod tests {}
