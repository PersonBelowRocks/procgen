use std::{future::Future, marker::PhantomData, sync::Arc};

use anymap::*;
use tokio::sync::{broadcast as bcst, mpsc, RwLock};

type Ev<Ctx, E> = (Arc<Ctx>, E);

pub trait BroadcastedEvent: 'static + Send + Clone {}
impl<T: 'static + Send + Clone> BroadcastedEvent for T {}

pub trait SingleEvent: 'static + Send {}
impl<T: 'static + Send> SingleEvent for T {}

#[async_trait::async_trait]
pub trait DispatcherContext: Send + Sync + 'static {
    async fn broadcast_event<E: BroadcastedEvent>(&self, event: E) -> bool;
    async fn fire_event<E: SingleEvent>(&self, event: E) -> bool;

    fn broadcast_event_blocking<E: BroadcastedEvent>(&self, event: E) -> bool;
    fn fire_event_blocking<E: SingleEvent>(&self, event: E) -> bool;
}

pub struct BcstEventProvider<Ctx, E> {
    rx: bcst::Receiver<Ev<Ctx, E>>,
    skipped: u64,
}

impl<Ctx, E: BroadcastedEvent> BcstEventProvider<Ctx, E> {
    pub async fn next(&mut self) -> Option<Ev<Ctx, E>> {
        use bcst::error::*;

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

pub struct SingleEventProvider<Ctx, E> {
    rx: mpsc::Receiver<Ev<Ctx, E>>,
}

impl<Ctx, E: SingleEvent> SingleEventProvider<Ctx, E> {
    pub async fn next(&mut self) -> Option<Ev<Ctx, E>> {
        self.rx.recv().await
    }
}

struct BcstSenderStorage<Ctx: DispatcherContext> {
    inner: RwLock<Map<dyn anymap::any::Any + Send + Sync>>,
    _ctx: PhantomData<Ctx>,
}

impl<Ctx: DispatcherContext> BcstSenderStorage<Ctx> {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Map::new()),
            _ctx: PhantomData,
        }
    }

    pub async fn receiver<E: BroadcastedEvent>(
        &self,
        buffer_size: usize,
    ) -> bcst::Receiver<Ev<Ctx, E>> {
        let mut guard = self.inner.write().await;
        if let Some(tx) = guard.get::<bcst::Sender<Ev<Ctx, E>>>() {
            tx.subscribe()
        } else {
            let (tx, rx) = bcst::channel::<Ev<Ctx, E>>(buffer_size);
            guard.insert(tx);
            rx
        }
    }

    pub async fn fire<E: BroadcastedEvent>(&self, ctx: Ctx, event: E) -> bool {
        self.inner
            .read()
            .await
            .get::<bcst::Sender<Ev<Ctx, E>>>()
            .and_then(|tx| tx.send((Arc::new(ctx), event)).ok())
            .is_some()
    }

    pub fn fire_blocking<E: BroadcastedEvent>(&self, ctx: Ctx, event: E) -> bool {
        self.inner
            .blocking_read()
            .get::<bcst::Sender<Ev<Ctx, E>>>()
            .and_then(|tx| tx.send((Arc::new(ctx), event)).ok())
            .is_some()
    }
}

struct SingleSenderStorage<Ctx: DispatcherContext> {
    inner: RwLock<Map<dyn anymap::any::Any + Send + Sync>>,
    _ctx: PhantomData<Ctx>,
}

impl<Ctx: DispatcherContext> SingleSenderStorage<Ctx> {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Map::new()),
            _ctx: PhantomData,
        }
    }

    pub async fn receiver<E: SingleEvent>(
        &self,
        buffer_size: usize,
    ) -> Option<mpsc::Receiver<Ev<Ctx, E>>> {
        let mut guard = self.inner.write().await;
        if guard.contains::<mpsc::Sender<Ev<Ctx, E>>>() {
            None
        } else {
            let (tx, rx) = mpsc::channel::<Ev<Ctx, E>>(buffer_size);
            guard.insert(tx);
            Some(rx)
        }
    }

    pub async fn fire<E: SingleEvent>(&self, ctx: Ctx, event: E) -> bool {
        if let Some(tx) = self.inner.read().await.get::<mpsc::Sender<Ev<Ctx, E>>>() {
            tx.send((Arc::new(ctx), event)).await.is_ok()
        } else {
            false
        }
    }

    pub fn fire_blocking<E: SingleEvent>(&self, ctx: Ctx, event: E) -> bool {
        if let Some(tx) = self.inner.blocking_read().get::<mpsc::Sender<Ev<Ctx, E>>>() {
            tx.blocking_send((Arc::new(ctx), event)).is_ok()
        } else {
            false
        }
    }
}

pub struct Dispatcher<Ctx: DispatcherContext>
where
    Self: Send + Sync,
{
    bcst_senders: BcstSenderStorage<Ctx>,
    single_senders: SingleSenderStorage<Ctx>,
    buffer_size: usize,
}

impl<Ctx: DispatcherContext> Dispatcher<Ctx> {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            bcst_senders: BcstSenderStorage::new(),
            single_senders: SingleSenderStorage::new(),
            buffer_size,
        }
    }

    pub async fn broadcast_handler<E: BroadcastedEvent>(&self) -> BcstEventProvider<Ctx, E> {
        let rx = self.bcst_senders.receiver(self.buffer_size).await;
        BcstEventProvider { rx, skipped: 0 }
    }

    pub async fn register_bcst<E, Fut, F>(&self, listener: F)
    where
        E: BroadcastedEvent,
        Fut: Future<Output = ()> + Send,
        F: Fn(Arc<Ctx>, E) -> Fut + Send + Sync + 'static,
    {
        let mut provider = self.broadcast_handler::<E>().await;

        tokio::spawn(async move {
            while let Some((ctx, ev)) = provider.next().await {
                listener(ctx, ev).await;
            }
        });
    }

    pub async fn broadcast_event<E: BroadcastedEvent>(&self, ctx: Ctx, event: E) -> bool {
        self.bcst_senders.fire(ctx, event).await
    }

    pub fn broadcast_event_blocking<E: BroadcastedEvent>(&self, ctx: Ctx, event: E) -> bool {
        self.bcst_senders.fire_blocking(ctx, event)
    }

    pub async fn single_handler<E: SingleEvent>(&self) -> Option<SingleEventProvider<Ctx, E>> {
        let rx = self.single_senders.receiver(self.buffer_size).await?;
        Some(SingleEventProvider { rx })
    }

    pub async fn register_single<E, Fut, F>(&self, listener: F) -> bool
    where
        E: SingleEvent,
        Fut: Future<Output = ()> + Send,
        F: Fn(Arc<Ctx>, E) -> Fut + Send + Sync + 'static,
    {
        if let Some(mut provider) = self.single_handler::<E>().await {
            tokio::spawn(async move {
                while let Some((ctx, ev)) = provider.next().await {
                    listener(ctx, ev).await;
                }
            });

            true
        } else {
            false
        }
    }

    pub async fn fire_event<E: SingleEvent>(&self, ctx: Ctx, event: E) -> bool {
        self.single_senders.fire(ctx, event).await
    }

    pub fn fire_event_blocking<E: SingleEvent>(&self, ctx: Ctx, event: E) -> bool {
        self.single_senders.fire_blocking(ctx, event)
    }
}
