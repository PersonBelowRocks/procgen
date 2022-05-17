use std::{
    collections::{hash_map::Entry, HashMap},
    future::Future,
    ops::{Bound, Range, RangeBounds},
    pin::Pin,
    task,
};

use anyhow::Result;
use threadpool::ThreadPool;
use tokio::sync::oneshot;

use crate::{
    chunk::{Chunk, IVec2},
    generate::{ChunkGenerator, HasGeneratorId},
};

#[derive(Debug)]
pub struct GeneratorArgs {
    pos: IVec2,
    y_bounds: Range<i32>,
}

impl GeneratorArgs {
    pub fn new<T: RangeBounds<i32>>(pos: IVec2, vertical_bounds: T) -> Self {
        let start = match vertical_bounds.start_bound() {
            Bound::Unbounded => panic!("range cannot be unbounded"),
            Bound::Excluded(&n) => n + 1,
            Bound::Included(&n) => n,
        };

        let end = match vertical_bounds.end_bound() {
            Bound::Unbounded => panic!("range cannot be unbounded"),
            Bound::Excluded(&n) => n,
            Bound::Included(&n) => n + 1,
        };

        assert!(start < end, "bound start must be smaller than bound end");

        Self {
            pos,
            y_bounds: start..end,
        }
    }

    pub fn bounds(&self) -> Range<i32> {
        self.y_bounds.clone()
    }

    pub fn max_y(&self) -> i32 {
        std::cmp::max(self.bounds().start, self.bounds().end)
    }

    pub fn min_y(&self) -> i32 {
        std::cmp::min(self.bounds().start, self.bounds().end)
    }

    pub fn pos(&self) -> IVec2 {
        self.pos
    }
}

#[derive(Clone, Copy, Hash, Debug)]
pub struct GenerationId(u32);

#[async_trait::async_trait]
pub trait GenerationHandle {
    async fn join(self) -> Result<Chunk>;
}

pub trait GeneratorExecutor {
    type Handle: GenerationHandle;
    fn is_parallel(&self) -> bool;
    fn submit(
        &self,
        generator: &'static dyn ChunkGenerator,
        args: GeneratorArgs,
    ) -> Result<Self::Handle>;
}

#[derive(Debug)]
pub struct ParallelExecutor {
    pool: ThreadPool,
}

#[derive(Debug)]
pub struct ParallelExecutorGenerationHandle {
    rx: oneshot::Receiver<Result<Chunk>>,
}

#[async_trait::async_trait]
impl GenerationHandle for ParallelExecutorGenerationHandle {
    async fn join(self) -> Result<Chunk> {
        self.rx.await.unwrap()
    }
}

impl ParallelExecutorGenerationHandle {
    fn new(rx: oneshot::Receiver<Result<Chunk>>) -> Self {
        Self { rx }
    }
}

impl GeneratorExecutor for ParallelExecutor {
    type Handle = ParallelExecutorGenerationHandle;

    fn is_parallel(&self) -> bool {
        true
    }

    fn submit(
        &self,
        generator: &'static dyn ChunkGenerator,
        args: GeneratorArgs,
    ) -> Result<Self::Handle> {
        let (tx, rx) = oneshot::channel::<Result<Chunk>>();

        self.pool.execute(move || {
            if range_contains_range_inclusive(&generator.bounds(), &args.bounds()) {
                tx.send({
                    let chunk_res = Chunk::try_new(
                        args.pos(),
                        args.max_y(),
                        args.min_y(),
                        generator.default_id(),
                    );

                    if let Some(mut chunk) = chunk_res {
                        generator.fill_chunk(&mut chunk);
                        Ok(chunk)
                    } else {
                        Err(anyhow::anyhow!("failed to initialize chunk"))
                    }
                })
                .unwrap();
            } else {
                tx.send(Err(anyhow::anyhow!(
                    "generator does not support generating chunks of requested vertical size"
                )))
                .unwrap();
            }
        });

        Ok(ParallelExecutorGenerationHandle::new(rx))
    }
}

impl ParallelExecutor {
    pub fn new() -> Self {
        Self {
            pool: ThreadPool::default(),
        }
    }
}

pub type GeneratorId = u32;

pub struct GeneratorManager<Exec: GeneratorExecutor> {
    executor: Exec,
    generators: HashMap<GeneratorId, &'static dyn ChunkGenerator>,
}

impl<Exec: GeneratorExecutor> std::fmt::Debug for GeneratorManager<Exec> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GeneratorManager {{")?;
        write!(
            f,
            "    executor: parallel: {},",
            self.executor.is_parallel()
        )?;
        write!(f, "    generators: {},", self.generators.len())?;
        write!(f, "}}")
    }
}

#[allow(dead_code)]
impl<Exec: GeneratorExecutor> GeneratorManager<Exec> {
    #[inline]
    pub fn new(executor: Exec) -> Self {
        Self {
            executor,
            generators: HashMap::new(),
        }
    }

    #[inline]
    pub fn add_generator<G>(&mut self, generator: G) -> Result<()>
    where
        G: ChunkGenerator + HasGeneratorId + 'static,
    {
        if let Entry::Vacant(e) = self.generators.entry(<G as HasGeneratorId>::GENERATOR_ID) {
            e.insert(Box::leak(Box::new(generator)));
            Ok(())
        } else {
            anyhow::bail!("generator with that ID already exists")
        }
    }

    #[inline]
    pub fn submit_chunk(
        &self,
        id: GeneratorId,
        args: GeneratorArgs,
    ) -> Option<<Exec as GeneratorExecutor>::Handle> {
        let generator = *self.generators.get(&id)?;

        let handle = self.executor.submit(generator, args).unwrap();
        Some(handle)
    }
}

#[inline]
fn range_contains_range_inclusive<T>(container: &Range<T>, contained: &Range<T>) -> bool
where
    T: Copy + Ord,
{
    use std::cmp::{max, min};

    let ctr = container;
    let ctd = contained;

    max(ctr.start, ctr.end) >= max(ctd.start, ctd.end)
        && min(ctr.start, ctr.end) <= min(ctd.start, ctd.end)
}

#[cfg(test)]
mod tests {
    use super::range_contains_range_inclusive;

    #[test]
    fn test_range_contains_range_inclusive() {
        assert!(range_contains_range_inclusive(&(-5..5), &(-5..4)));
        assert!(!range_contains_range_inclusive(&(-5..4), &(-5..5)));
        assert!(range_contains_range_inclusive(&(-5..5), &(-5..5)));
        assert!(range_contains_range_inclusive(&(-5..5), &(-4..5)));
        assert!(!range_contains_range_inclusive(&(-5..5), &(-6..5)));
        assert!(!range_contains_range_inclusive(&(-4..4), &(-5..4)));
    }
}
