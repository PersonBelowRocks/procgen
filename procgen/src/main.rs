use bracket_noise::prelude::*;
use common::{
    generation::{FactoryParameters, GenerationArgs},
    BlockId, Chunk, Spaces,
};
use flate2::Compression;
use generation::{ChunkGenerator, GeneratorFactory};
use vol::Volume;

use crate::runtime::{net::Compressor, server::Server};

extern crate downcast_rs as dc;
extern crate nalgebra as na;
extern crate procgen_common as common;
extern crate thiserror as te;
extern crate volume as vol;

mod generation;
#[allow(dead_code)]
mod runtime;
mod util;

struct MockGenFactory;

impl GeneratorFactory for MockGenFactory {
    type Generator = MockGenerator;

    fn create(&self, params: FactoryParameters<'_>) -> Self::Generator {
        MockGenerator {
            min_height: params.min_height,
            max_height: params.max_height,
            default_id: params.default,
            noise: FastNoise::new(),
        }
    }
}

struct MockGenerator {
    min_height: i32,
    max_height: i32,
    default_id: BlockId,
    noise: FastNoise,
}

impl ChunkGenerator for MockGenerator {
    const NAME: &'static str = "BIG_FART";

    type Factory = MockGenFactory;

    fn generate(&self, args: &GenerationArgs) -> anyhow::Result<Chunk> {
        let mut chunk = Chunk::new(self.default_id, args.pos, self.min_height, self.max_height);

        for x in 0..16 {
            for z in 0..16 {
                let mut ws_block_pos = (na::vector![args.pos.x as f32, args.pos.y as f32] * 16.0)
                    + na::vector![x as f32, z as f32];
                ws_block_pos /= 75.0;
                // let height = (ws_block_pos / 10.0).magnitude().sin() * 10.0;

                // let cs_block_pos_2d = na::vector![x as f32, z as f32];

                let height = self.noise.get_noise(ws_block_pos.x, ws_block_pos.y) * 20.0;

                // println!("{height}");

                for y in self.min_height..(height.floor() as i32) {
                    // println!("{y}");
                    chunk.set(Spaces::Cs([x, y, z]), 1.into());
                }
            }
        }

        Ok(chunk)
    }

    fn factory() -> Self::Factory {
        MockGenFactory
    }
}

#[tokio::main]
async fn main() {
    use common::packets::Packet;

    env_logger::init();

    let mut chunk = Chunk::new(20.into(), na::vector![21, 24], -64, 320);
    chunk.set(Spaces::Cs([8i32, -60, 4]), 42.into());
    chunk.set(Spaces::Cs([1i32, 310, 2]), 42.into());

    let packet = common::packets::ReplyChunk {
        request_id: 400.into(),
        chunk,
    };

    let compressor = Compressor::new(Compression::best());

    let mut compressed_buf = Vec::new();
    compressor
        .write(&packet.to_bincode().unwrap(), &mut compressed_buf)
        .await
        .unwrap();

    // println!(
    //     "{}",
    //     compressed_buf[8..] // skip the compression header
    //         .iter()
    //         .map(|e| e.to_string() + "u")
    //         .reduce(|accum, item| { format!("{}, {}", accum, item) })
    //         .unwrap()
    // );

    let mut server = Server::new(runtime::server::ServerParams {
        addr: "0.0.0.0:4432".parse().unwrap(),
        compression: Compression::best(),
        coarsening: 50,
    })
    .await;
    server.add_generator::<MockGenerator>().await;

    server.run().await;

    loop {
        std::hint::spin_loop()
    }
}
