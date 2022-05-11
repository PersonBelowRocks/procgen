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

    let mut server = net::server::Server::new()
        .with_version(net::protocol::ProtocolVersion::V1)
        .with_compression_threshold(256);

    server.bind("0.0.0.0:4321".parse().unwrap()).await.unwrap();
    server.run().await.unwrap();
}

#[cfg(test)]
mod tests {}
