use common::{Bounded, Parameters, VoxelVolume};
use flate2::Compression;
use generation::{RegionGenerator, RegionGeneratorFactory};
use runtime::server::{Server, ServerParams};
use vol::Volume;

extern crate downcast_rs as dc;
extern crate nalgebra as na;
extern crate procgen_common as common;
extern crate thiserror as te;
extern crate volume as vol;

mod generation;
#[allow(dead_code)]
mod runtime;
mod util;

struct DemoGenerator;

#[derive(Debug, te::Error)]
#[error("Example error message (generator)")]
struct DemoGeneratorError;

impl RegionGenerator for DemoGenerator {
    type Error = DemoGeneratorError;

    fn generate(
        &self,
        vol: &mut VoxelVolume<Bounded>,
        _ctx: crate::generation::GenerationContext,
    ) -> Result<(), Self::Error> {
        let min = vol.bounding_box().min();
        let max = vol.bounding_box().max();

        for x in min.x..max.x {
            for z in min.z..max.z {
                vol.set([x, min.y, z].into(), 100.into());
            }
        }

        Ok(())
    }
}

struct DemoFactory;

#[derive(Debug, te::Error)]
#[error("Example error message (factory)")]
struct DemoFactoryError;

impl RegionGeneratorFactory for DemoFactory {
    type Error = DemoFactoryError;
    type Generator = DemoGenerator;

    fn new_generator(&self, _params: &Parameters) -> Result<Self::Generator, Self::Error> {
        Ok(DemoGenerator)
    }

    fn name(&self) -> String {
        "DEMO".into()
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let params = ServerParams {
        addr: "0.0.0.0:4432".parse().unwrap(),
        compression: Compression::best(),
        coarsening: 100,
    };

    let mut server = Server::new(params).await;
    server.add_region_generator(DemoFactory).await;
    server.run().await;

    loop {
        std::hint::spin_loop()
    }
}
