use std::{collections::HashMap, net::SocketAddrV4, sync::Arc};
use threadpool::ThreadPool;
use tokio::{net::TcpListener, sync::oneshot::Receiver};

use crate::{chunk::Chunk, generate::Generator};

use super::protocol::{GeneratorId, ProtocolVersion, RequestId};

pub struct Server {
    reactor: Option<()>,
    listener: Option<TcpListener>,
    version: ProtocolVersion,
    compression_threshold: Option<usize>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            reactor: None,
            listener: None,
            version: Default::default(),
            compression_threshold: None,
        }
    }

    pub fn with_version(mut self, version: ProtocolVersion) -> Self {
        self.version = version;
        self
    }

    pub fn with_compression_threshold(mut self, threshold: usize) -> Self {
        self.compression_threshold = Some(threshold);
        self
    }

    pub async fn bind(&mut self, address: SocketAddrV4) -> anyhow::Result<()> {
        let listener = TcpListener::bind(address).await?;
        self.listener = Some(listener);

        Ok(())
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        todo!()
    }
}

struct PacketReactor {
    generator_manager: Arc<GeneratorManager>,
}

impl PacketReactor {
    pub fn new(generator_manager: Arc<GeneratorManager>) -> Self {
        Self { generator_manager }
    }
}

struct GeneratorManager {
    thread_pool: ThreadPool,
    generators: HashMap<GeneratorId, Arc<dyn Generator>>,
    requests: Vec<Receiver<GenerationReport>>,
}

#[derive(thiserror::Error, Debug)]
pub enum GeneratorError {
    #[error("no generator found with that id")]
    NoSuchGenerator,
    #[error("attempted to add generator with ID that already exists")]
    GeneratorAlreadyExists,
}

#[derive(Debug)]
struct GenerationReport {
    request_id: RequestId,
    chunk: Chunk,
}

impl GenerationReport {
    fn new(request_id: RequestId, chunk: Chunk) -> Self {
        Self { request_id, chunk }
    }
}

#[allow(dead_code)]
impl GeneratorManager {
    pub fn new() -> Self {
        let thread_pool = threadpool::Builder::new()
            .thread_name("generator-worker".into())
            .build(); // TODO: number of threads should be configurable

        Self {
            thread_pool,
            requests: Vec::new(),
            generators: HashMap::new(),
        }
    }

    pub fn add_generator<T: Generator + 'static>(
        &mut self,
        generator_id: GeneratorId,
        gen: T,
    ) -> anyhow::Result<()> {
        if self.generators.contains_key(&generator_id) {
            return Err(GeneratorError::GeneratorAlreadyExists.into());
        }

        self.generators.insert(generator_id, Arc::new(gen));

        Ok(())
    }

    pub fn submit_chunk(
        &mut self,
        generator_id: GeneratorId,
        request_id: RequestId,
        mut chunk: Chunk,
    ) -> anyhow::Result<()> {
        let generator = self
            .generators
            .get(&generator_id)
            .cloned()
            .ok_or(GeneratorError::NoSuchGenerator)?;

        let (tx, rx) = tokio::sync::oneshot::channel::<GenerationReport>();
        self.requests.push(rx);

        self.thread_pool.execute(move || {
            generator.fill_chunk(&mut chunk);
            tx.send(GenerationReport::new(request_id, chunk)).unwrap();
        });

        Ok(())
    }

    pub fn get_chunks(&mut self) -> Option<Vec<GenerationReport>> {
        let mut reports = Vec::new();
        self.requests = self
            .requests
            .drain(..)
            .filter_map(|mut rx| {
                if let Ok(report) = rx.try_recv() {
                    reports.push(report);
                    None
                } else {
                    Some(rx)
                }
            })
            .collect::<Vec<_>>();

        if reports.is_empty() {
            None
        } else {
            Some(reports)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{block::BlockId, chunk::CHUNK_SECTION_SIZE};

    use super::*;

    fn example_chunk() -> Chunk {
        Chunk::try_new(na::vector![2, 2], 320, -64, BlockId::from(0)).unwrap()
    }

    #[test]
    fn generator_manager() {
        struct MyGenerator;

        impl Generator for MyGenerator {
            fn fill_chunk(&self, chunk: &mut Chunk) {
                for x in 0..CHUNK_SECTION_SIZE as i32 {
                    for z in 0..CHUNK_SECTION_SIZE as i32 {
                        chunk
                            .set(na::vector![x, chunk.min_y(), z], 42.into())
                            .unwrap();
                    }
                }
            }
        }

        const GENERATOR_ID: GeneratorId = 100;
        const REQUEST_ID: RequestId = 54;

        let mut manager = GeneratorManager::new();
        manager.add_generator(GENERATOR_ID, MyGenerator).unwrap();
        manager
            .submit_chunk(GENERATOR_ID, REQUEST_ID, example_chunk())
            .unwrap();

        loop {
            if let Some(mut chunks) = manager.get_chunks() {
                assert_eq!(chunks.len(), 1);

                let only = chunks.pop().unwrap();

                assert_eq!(only.request_id, REQUEST_ID);

                for x in 0..CHUNK_SECTION_SIZE as i32 {
                    for z in 0..CHUNK_SECTION_SIZE as i32 {
                        let slot = only
                            .chunk
                            .get(na::vector![x, only.chunk.min_y(), z])
                            .unwrap();
                        assert_eq!(slot, &42.into())
                    }
                }
                return;
            }
        }
    }
}
