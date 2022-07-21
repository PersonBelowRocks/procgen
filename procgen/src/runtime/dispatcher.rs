use std::{marker::PhantomData, sync::Arc};

use anymap::*;
use tokio::sync::{broadcast::*, Mutex};

type Ev<Ctx, E> = (Arc<Ctx>, E);

pub trait Event: 'static + Send + Clone {}
impl<T: 'static + Send + Clone> Event for T {}

#[async_trait::async_trait]
pub trait DispatcherContext: Send + Sync + 'static {
    async fn fire_event<E: Event>(&self, event: E) -> bool;
}

pub struct EventProvider<Ctx, E> {
    rx: Receiver<Ev<Ctx, E>>,
    skipped: u64,
}

impl<Ctx, E: Event> EventProvider<Ctx, E> {
    pub async fn next(&mut self) -> Option<Ev<Ctx, E>> {
        use error::*;

        match self.rx.recv().await {
            Ok(event) => Some(event),
            Err(error) => match error {
                RecvError::Closed => None,
                RecvError::Lagged(_skipped) => todo!(),
            },
        }
    }

    pub fn skipped(&self) -> u64 {
        self.skipped
    }
}

struct SenderStorage<Ctx: DispatcherContext> {
    inner: Mutex<Map<dyn anymap::any::Any + Send>>,
    _ctx: PhantomData<Ctx>,
}

impl<Ctx: DispatcherContext> SenderStorage<Ctx> {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Map::new()),
            _ctx: PhantomData,
        }
    }

    pub async fn receiver<E: Event>(&self, buffer_size: usize) -> Receiver<Ev<Ctx, E>> {
        let mut guard = self.inner.lock().await;
        if let Some(tx) = guard.get::<Sender<Ev<Ctx, E>>>() {
            tx.subscribe()
        } else {
            let (tx, rx) = channel::<Ev<Ctx, E>>(buffer_size);
            guard.insert(tx);
            rx
        }
    }

    pub async fn fire<E: Event>(&self, ctx: Ctx, event: E) -> bool {
        self.inner
            .lock()
            .await
            .get::<Sender<Ev<Ctx, E>>>()
            .and_then(|tx| tx.send((Arc::new(ctx), event)).ok())
            .is_some()
    }
}

pub struct Dispatcher<Ctx: DispatcherContext>
where
    Self: Send + Sync,
{
    senders: SenderStorage<Ctx>,
    buffer_size: usize,
    _ctx: PhantomData<Ctx>,
}

impl<Ctx: DispatcherContext> Dispatcher<Ctx> {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            senders: SenderStorage::new(),
            buffer_size,
            _ctx: PhantomData,
        }
    }

    pub async fn handler<E: Event>(&self) -> EventProvider<Ctx, E> {
        let rx = self.senders.receiver(self.buffer_size).await;
        EventProvider { rx, skipped: 0 }
    }

    pub async fn fire_event<E: Event>(&self, ctx: Ctx, event: E) -> bool {
        self.senders.fire(ctx, event).await
    }
}
