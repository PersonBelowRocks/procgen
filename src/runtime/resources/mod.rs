use std::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex, MutexGuard,
};

use threadpool::ThreadPool;

use crate::{chunk::Chunk, generation::GenerationArgs};

use super::{components::Generator, events::FinishedGenerateChunkEvent, RequestIdent};

pub struct ChunkGenerationOutput(Mutex<Receiver<FinishedGenerateChunkEvent>>);

impl From<Receiver<FinishedGenerateChunkEvent>> for ChunkGenerationOutput {
    fn from(rx: Receiver<FinishedGenerateChunkEvent>) -> Self {
        Self(Mutex::new(rx))
    }
}

impl ChunkGenerationOutput {
    /// Lock the internal mutex and poll the channel once.
    /// This locks once every call so try not to call it too much.
    pub fn poll(&self) -> Option<FinishedGenerateChunkEvent> {
        self.0.lock().unwrap().try_recv().ok()
    }

    /// Locks the internal mutex and returns an iterator over the channel.
    /// This is preferred over [`ChunkGenerationOutput::poll`] whenever possible due to performance.
    pub fn iter_poll(&self) -> ChunkgenIter<'_> {
        ChunkgenIter {
            channel: self.0.lock().unwrap(),
        }
    }
}

pub struct ChunkgenIter<'a> {
    channel: MutexGuard<'a, Receiver<FinishedGenerateChunkEvent>>,
}

impl<'a> Iterator for ChunkgenIter<'a> {
    type Item = FinishedGenerateChunkEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.channel.try_recv().ok()
    }
}

pub struct GenerationPool {
    pool: Mutex<ThreadPool>,
    tx: Mutex<Sender<FinishedGenerateChunkEvent>>,
}

impl GenerationPool {
    pub fn new() -> (Self, ChunkGenerationOutput) {
        let (tx, rx) = mpsc::channel();

        (
            Self {
                pool: Mutex::new(ThreadPool::default()),
                tx: Mutex::new(tx),
            },
            rx.into(),
        )
    }

    pub fn submit(&self, args: GenerationArgs, ident: RequestIdent, generator: Generator) {
        let tx_clone = self.tx.lock().unwrap().clone();

        self.pool.lock().unwrap().execute(move || {
            let chunk = process_request(args, generator);
            tx_clone
                .send(FinishedGenerateChunkEvent::new(chunk, ident))
                .expect("failed to send chunk from the GenerationPool");
        });
    }
}

#[inline]
fn process_request(args: GenerationArgs, generator: Generator) -> Chunk {
    let mut chunk = Chunk::from_args(args);
    // Just be aware that this function call is probably going to be our main bottleneck!
    // Depending on the generator this might end up doing A LOT of stuff.
    generator
        .generate(&mut chunk)
        .unwrap_or_else(|e| println!("generator couldn't generate for args {args:?}: {e}"));
    chunk
}
