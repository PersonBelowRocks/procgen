use std::collections::HashMap;
use std::sync::Arc;

use procgen_common::packets;
use procgen_common::Bounded;
use procgen_common::GeneratorId;
use procgen_common::Unbounded;
use procgen_common::VoxelVolume;
use rayon::ThreadPool;
use rayon::ThreadPoolBuilder;
use tokio::sync::RwLock;

use crate::runtime::dispatcher::DispatcherContext;
use crate::runtime::dispatcher::SingleEventProvider;
use crate::runtime::events::Context;
use crate::runtime::events::FinishedGeneratingBrushEvent;
use crate::runtime::events::FinishedGeneratingRegionEvent;
use crate::runtime::events::GenerateBrushEvent;
use crate::runtime::events::GenerateRegionEvent;

use super::BrushGeneratorFactory;
use super::DynamicBrushGenerator;
use super::DynamicBrushGeneratorFactory;
use super::DynamicRegionGenerator;
use super::DynamicRegionGeneratorFactory;
use super::GenBrushRequest;
use super::GenRegionRequest;
use super::GenerationContext;
use super::RegionGeneratorFactory;

#[derive(Debug, te::Error)]
#[error("Generator not found with ID {0}")]
pub struct ManagerSubmitError(GeneratorId);

#[allow(dead_code)]
pub struct GeneratorManager {
    region_gen_factories: Arc<RwLock<HashMap<String, Box<dyn DynamicRegionGeneratorFactory>>>>,
    brush_gen_factories: Arc<RwLock<HashMap<String, Box<dyn DynamicBrushGeneratorFactory>>>>,
    pool: Arc<ThreadPool>,
}

#[derive(te::Error, Debug)]
pub enum GenerationError {
    #[error("generator not found with name {0}")]
    GeneratorNotFound(String),
    #[error("internal factory error: {0}")]
    InternalFactoryError(#[from] anyhow::Error),
}

impl GeneratorManager {
    pub fn new() -> Self {
        Self {
            region_gen_factories: RwLock::new(HashMap::new()).into(),
            brush_gen_factories: RwLock::new(HashMap::new()).into(),
            pool: ThreadPoolBuilder::new()
                .num_threads(0)
                .build()
                .unwrap()
                .into(),
        }
    }

    pub async fn add_region_factory<Factory: RegionGeneratorFactory>(&self, factory: Factory) {
        self.region_gen_factories
            .write()
            .await
            .insert(factory.name(), Box::new(factory));
    }

    pub async fn add_brush_factory<Factory: BrushGeneratorFactory>(&self, factory: Factory) {
        self.brush_gen_factories
            .write()
            .await
            .insert(factory.name(), Box::new(factory));
    }

    #[allow(dead_code)]
    pub async fn generate_brush(
        &self,
        request: &GenBrushRequest,
    ) -> Result<Box<dyn DynamicBrushGenerator>, GenerationError> {
        let name = request.parameters.generator_name();

        match self.brush_gen_factories.read().await.get(name) {
            Some(factory) => Ok(factory.new_generator(&request.parameters)?),
            None => Err(GenerationError::GeneratorNotFound(name.into())),
        }
    }

    #[allow(dead_code)]
    pub async fn generate_region(
        &self,
        request: &GenRegionRequest,
    ) -> Result<Box<dyn DynamicRegionGenerator>, GenerationError> {
        let name = request.parameters.generator_name();

        match self.region_gen_factories.read().await.get(name) {
            Some(factory) => Ok(factory.new_generator(&request.parameters)?),
            None => Err(GenerationError::GeneratorNotFound(name.into())),
        }
    }

    pub(crate) async fn generate_brush_listener(
        &self,
        mut provider: SingleEventProvider<Context, GenerateBrushEvent>,
    ) {
        let pool = self.pool.clone();
        let factories = self.brush_gen_factories.clone();

        tokio::spawn(async move {
            while let Some((ctx, event)) = provider.next().await {
                if let Some(factory) = factories
                    .read()
                    .await
                    .get(event.request.parameters.generator_name())
                {
                    let generator = match factory.new_generator(&event.request.parameters) {
                        Ok(generator) => Some(generator),
                        Err(error) => {
                            let err = packets::ProtocolErrorKind::FactoryError {
                                generator_name: event
                                    .request
                                    .parameters
                                    .generator_name()
                                    .to_string(),
                                request_id: event.request_id,
                                details: error.to_string(),
                            };

                            event
                                .connection
                                .send_packet(&packets::ProtocolError::gentle(err))
                                .await
                                .unwrap();

                            None
                        }
                    };

                    if let Some(generator) = generator {
                        let generation_ctx = GenerationContext { pool: pool.clone() };
                        let ctx = ctx.clone();

                        pool.spawn(move || {
                            let mut volume = VoxelVolume::<Unbounded>::new();
                            let result =
                                generator.generate(&mut volume, event.request.pos, generation_ctx);

                            let completion_event = FinishedGeneratingBrushEvent {
                                request_id: event.request_id,
                                volume: result.map(|_| volume),
                                connection: event.connection,
                                pos: event.request.pos,
                                generator_name: event
                                    .request
                                    .parameters
                                    .generator_name()
                                    .to_string(),
                            };

                            ctx.fire_event_blocking(completion_event);
                        });
                    }
                } else {
                    let err = packets::ProtocolErrorKind::GeneratorNotFound {
                        generator_name: event.request.parameters.generator_name,
                        request_id: event.request_id,
                    };
                    event
                        .connection
                        .send_packet(&packets::ProtocolError::gentle(err))
                        .await
                        .unwrap();
                }
            }
        });
    }

    pub(crate) async fn generate_region_listener(
        &self,
        mut provider: SingleEventProvider<Context, GenerateRegionEvent>,
    ) {
        let pool = self.pool.clone();
        let factories = self.region_gen_factories.clone();

        tokio::spawn(async move {
            while let Some((ctx, event)) = provider.next().await {
                if let Some(factory) = factories
                    .read()
                    .await
                    .get(event.request.parameters.generator_name())
                {
                    let generator = match factory.new_generator(&event.request.parameters) {
                        Ok(generator) => Some(generator),
                        Err(error) => {
                            let err = packets::ProtocolErrorKind::FactoryError {
                                generator_name: event
                                    .request
                                    .parameters
                                    .generator_name()
                                    .to_string(),
                                request_id: event.request_id,
                                details: error.to_string(),
                            };

                            event
                                .connection
                                .send_packet(&packets::ProtocolError::gentle(err))
                                .await
                                .unwrap();

                            None
                        }
                    };

                    if let Some(generator) = generator {
                        let generation_ctx = GenerationContext { pool: pool.clone() };
                        let ctx = ctx.clone();

                        pool.spawn(move || {
                            let mut volume = VoxelVolume::<Bounded>::new(event.request.region);
                            let result = generator.generate(&mut volume, generation_ctx);

                            let completion_event = FinishedGeneratingRegionEvent {
                                request_id: event.request_id,
                                volume: result.map(|_| volume),
                                connection: event.connection,
                                generator_name: event
                                    .request
                                    .parameters
                                    .generator_name()
                                    .to_string(),
                            };

                            ctx.fire_event_blocking(completion_event);
                        });
                    }
                } else {
                    let err = packets::ProtocolErrorKind::GeneratorNotFound {
                        generator_name: event.request.parameters.generator_name,
                        request_id: event.request_id,
                    };
                    event
                        .connection
                        .send_packet(&packets::ProtocolError::gentle(err))
                        .await
                        .unwrap();
                }
            }
        });
    }
}
