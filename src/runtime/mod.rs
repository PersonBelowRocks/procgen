//! This module contains code related to the generator server's runtime. This includes the ECS world(s), ECS schedule(s), and async runtime for netcode.

mod components;
mod events;
mod resources;
mod systems;

use ecs::prelude::*;
use tokio::runtime::{Builder, Runtime};

use crate::generation::ChunkGenerator;

use self::components::{Generator, GeneratorName};

mod labels {
    pub(super) static PRE_TICK: &str = "STAGE_PRE_TICK";
    pub(super) static TICK: &str = "STAGE_TICK";
    pub(super) static POST_TICK: &str = "STAGE_POST_TICK";
    pub(super) static SETUP: &str = "STAGE_SETUP";
}

#[derive(Copy, Clone, Debug, Hash, PartialEq)]
pub struct RequestIdent {
    request_id: u32,
    caller_id: u32,
}

impl RequestIdent {
    pub fn new(request_id: u32, caller_id: u32) -> Self {
        Self {
            request_id,
            caller_id,
        }
    }
}

pub type GeneratorManager = Vec<()>;

// TODO: we should maybe have some kind of Runtime struct to store all the data and state for the runtime. We'll interact with this struct to add generators and stuff.
pub struct GenRuntime {
    sched: Schedule,
    world: World,
    async_rt: Runtime,
}

impl GenRuntime {
    pub fn new() -> Self {
        Self {
            sched: {
                let mut schedule = Schedule::default()
                    // The PRE_TICK stage is where requests from clients are collected from the async background networking worker.
                    // Requests will be processed in the next stage (TICK).
                    .with_stage(labels::PRE_TICK, SystemStage::parallel())
                    // The TICK stage is where chunks are submitted for generation to the async background thread pool.
                    // Generators will run in this pool alongside the ECS runtime, and will be collected when available.
                    .with_stage_after(labels::PRE_TICK, labels::TICK, SystemStage::parallel())
                    // The POST_TICK stage is where finished chunks are going to be submitted to the async background networking worker.
                    // Here they will be sent out to the clients that requested them.
                    .with_stage_after(labels::TICK, labels::POST_TICK, SystemStage::parallel());

                schedule.add_system_to_stage(labels::TICK, systems::generate_chunks);
                schedule.add_system_to_stage(labels::POST_TICK, systems::collect_chunks);

                schedule
            },
            world: { World::new() },
            async_rt: { Builder::new_multi_thread().enable_all().build().unwrap() },
        }
    }

    fn setup(&mut self) -> anyhow::Result<()> {
        let mut setup_schedule = {
            let mut sched =
                Schedule::default().with_stage(labels::SETUP, SystemStage::single_threaded());

            sched.add_system_to_stage(labels::SETUP, systems::setup);
            sched
        };

        // Run the setup schedule once for our world to initialize resources and stuff.
        setup_schedule.run_once(&mut self.world);

        Ok(())
    }

    fn add_generator<G: ChunkGenerator + 'static>(&mut self, generator: G) {
        self.world
            .spawn()
            .insert(GeneratorName::from(G::NAME.to_string()))
            .insert(Generator::from(generator));
    }

    /// Hijacks current thread to run the runtime.
    pub fn hijack(mut self) -> ! {
        self.setup().expect("Failed to setup runtime");

        todo!()
    }
}
