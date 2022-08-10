mod manager;
pub use manager::*;
use rayon::ThreadPool;

use std::error::Error;
use std::sync::Arc;

use procgen_common::packets;
use procgen_common::Bounded;
use procgen_common::Parameters;
use procgen_common::Unbounded;
use procgen_common::VoxelVolume;
use vol::BoundingBox;

#[allow(dead_code)]
pub struct GenerationContext {
    pool: Arc<ThreadPool>,
}

pub trait BrushGenerator: DynamicBrushGenerator {
    type Error: Error + Send + Sync + 'static;

    fn generate(
        &self,
        vol: &mut VoxelVolume<Unbounded>,
        pos: na::Vector3<i64>,
        ctx: GenerationContext,
    ) -> Result<(), Self::Error>;
}

pub trait DynamicBrushGenerator: Send {
    fn generate(
        &self,
        vol: &mut VoxelVolume<Unbounded>,
        pos: na::Vector3<i64>,
        ctx: GenerationContext,
    ) -> anyhow::Result<()>;
}

impl<G: BrushGenerator> DynamicBrushGenerator for G {
    fn generate(
        &self,
        vol: &mut VoxelVolume<Unbounded>,
        pos: na::Vector3<i64>,
        ctx: GenerationContext,
    ) -> anyhow::Result<()> {
        <Self as BrushGenerator>::generate(self, vol, pos, ctx)?;
        Ok(())
    }
}

pub trait BrushGeneratorFactory: 'static + Send + Sync {
    type Generator: BrushGenerator;
    type Error: Error + Send + Sync + 'static;

    fn new_generator(&self, params: &Parameters) -> Result<Self::Generator, Self::Error>;
    fn name(&self) -> String;
}

pub(crate) trait DynamicBrushGeneratorFactory: 'static + Send + Sync {
    fn new_generator(&self, params: &Parameters) -> anyhow::Result<Box<dyn DynamicBrushGenerator>>;
}

impl<G: BrushGeneratorFactory + 'static + Send + Sync> DynamicBrushGeneratorFactory for G {
    fn new_generator(&self, params: &Parameters) -> anyhow::Result<Box<dyn DynamicBrushGenerator>> {
        Ok(Box::new(<Self as BrushGeneratorFactory>::new_generator(
            self, params,
        )?))
    }
}

pub trait RegionGenerator: DynamicRegionGenerator {
    type Error: Error + Send + Sync + 'static;

    fn generate(
        &self,
        vol: &mut VoxelVolume<Bounded>,
        ctx: GenerationContext,
    ) -> Result<(), Self::Error>;
}

pub trait DynamicRegionGenerator: Send {
    fn generate(
        &self,
        vol: &mut VoxelVolume<Bounded>,
        ctx: GenerationContext,
    ) -> anyhow::Result<()>;
}

impl<G: RegionGenerator> DynamicRegionGenerator for G {
    fn generate(
        &self,
        vol: &mut VoxelVolume<Bounded>,
        ctx: GenerationContext,
    ) -> anyhow::Result<()> {
        <Self as RegionGenerator>::generate(self, vol, ctx)?;
        Ok(())
    }
}

pub trait RegionGeneratorFactory: 'static + Send + Sync {
    type Generator: RegionGenerator;
    type Error: Error + Send + Sync + 'static;

    fn new_generator(&self, params: &Parameters) -> Result<Self::Generator, Self::Error>;
    fn name(&self) -> String;
}

pub(crate) trait DynamicRegionGeneratorFactory: 'static + Send + Sync {
    fn new_generator(&self, params: &Parameters)
        -> anyhow::Result<Box<dyn DynamicRegionGenerator>>;
}

impl<G: RegionGeneratorFactory + 'static + Send + Sync> DynamicRegionGeneratorFactory for G {
    fn new_generator(
        &self,
        params: &Parameters,
    ) -> anyhow::Result<Box<dyn DynamicRegionGenerator>> {
        Ok(Box::new(<Self as RegionGeneratorFactory>::new_generator(
            self, params,
        )?))
    }
}

#[derive(Debug)]
pub struct GenBrushRequest {
    pub pos: na::Vector3<i64>,
    pub parameters: Parameters,
}

impl From<packets::GenerateBrush> for GenBrushRequest {
    fn from(packet: packets::GenerateBrush) -> Self {
        GenBrushRequest {
            pos: packet.pos,
            parameters: packet.params,
        }
    }
}

#[derive(Debug)]
pub struct GenRegionRequest {
    pub region: BoundingBox,
    pub parameters: Parameters,
}

impl From<packets::GenerateRegion> for GenRegionRequest {
    fn from(packet: packets::GenerateRegion) -> Self {
        GenRegionRequest {
            region: packet.bounds.into(),
            parameters: packet.params,
        }
    }
}
