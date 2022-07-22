use std::sync::Arc;

use procgen_common::packets::{self, DowncastPacket, Packet};
use tokio::sync::Mutex;

use super::{
    dispatcher::{self, BcstEventProvider, Dispatcher, DispatcherContext, SingleEventProvider},
    net::{Connection, Networker},
    server::{GenerationResult, GeneratorManager},
    util::RequestIdent,
};

pub async fn defaults(dispatcher: &Dispatcher<Context>) {
    let provider = dispatcher.broadcast_handler::<IncomingPacket>().await;
    tokio::spawn(handle_incoming_packet(provider));

    let provider = dispatcher
        .broadcast_handler::<ReceivedPacket<packets::GenerateRegion>>()
        .await;
    tokio::spawn(handle_generate_region(provider));

    let provider = dispatcher
        .broadcast_handler::<ReceivedPacket<packets::GenerateChunk>>()
        .await;
    tokio::spawn(handle_generate_chunk(provider));

    let provider = dispatcher
        .broadcast_handler::<ReceivedPacket<packets::AddGenerator>>()
        .await;
    tokio::spawn(handle_add_generator(provider));

    let provider = dispatcher.single_handler::<ChunkFinished>().await.unwrap();
    tokio::spawn(handle_generated_chunk(provider));
}

#[derive(Clone)]
pub struct Context {
    pub dispatcher: Arc<Dispatcher<Self>>,
    pub generators: Arc<Mutex<GeneratorManager>>,
    pub networker: Networker,
}

#[async_trait::async_trait]
impl dispatcher::DispatcherContext for Context {
    async fn broadcast_event<E: dispatcher::BroadcastedEvent>(&self, event: E) -> bool {
        self.dispatcher.broadcast_event(self.clone(), event).await
    }

    async fn fire_event<E: dispatcher::SingleEvent>(&self, event: E) -> bool {
        self.dispatcher.fire_event(self.clone(), event).await
    }
}

#[derive(Clone)]
pub struct IncomingPacket {
    pub connection: Connection,
    pub packet: Arc<dyn DowncastPacket>,
}

#[derive(Clone)]
pub struct ReceivedPacket<P: Packet> {
    connection: Connection,
    packet: Arc<P>,
}

pub struct ChunkFinished {
    pub result: GenerationResult,
}

type Prov<E> = BcstEventProvider<Context, E>;
type PRecv<P> = Prov<ReceivedPacket<P>>;

async fn handle_incoming_packet(mut provider: Prov<IncomingPacket>) {
    while let Some((ctx, event)) = provider.next().await {
        if let Some(packet) = event.packet.downcast_ref::<packets::GenerateRegion>() {
            let packet = ReceivedPacket {
                connection: event.connection.clone(),
                packet: Arc::new(packet.clone()),
            };

            ctx.broadcast_event(packet).await;
        }

        if let Some(packet) = event.packet.downcast_ref::<packets::GenerateChunk>() {
            let packet = ReceivedPacket {
                connection: event.connection.clone(),
                packet: Arc::new(packet.clone()),
            };

            ctx.broadcast_event(packet).await;
        }

        if let Some(packet) = event.packet.downcast_ref::<packets::AddGenerator>() {
            let packet = ReceivedPacket {
                connection: event.connection.clone(),
                packet: Arc::new(packet.clone()),
            };

            ctx.broadcast_event(packet).await;
        }
    }
}

async fn handle_generate_region(mut provider: PRecv<packets::GenerateRegion>) {
    while let Some((_ctx, ev)) = provider.next().await {
        let packet = ev.packet;
        log::info!("Received request to generate region: {packet:?}")
    }
}

async fn handle_generate_chunk(mut provider: PRecv<packets::GenerateChunk>) {
    while let Some((ctx, ev)) = provider.next().await {
        let packet = ev.packet;
        let request_ident = RequestIdent::new(packet.request_id, ev.connection.id());

        {
            if let Err(error) = ctx
                .generators
                .lock()
                .await
                .submit_chunk(request_ident, packet.generator_id, packet.args())
                .await
            {
                log::error!("Request {request_ident:?} failed when submitting chunk for generation: {error}");
            }
        }
    }
}

async fn handle_add_generator(mut provider: PRecv<packets::AddGenerator>) {
    while let Some((ctx, ev)) = provider.next().await {
        let packet = ev.packet;

        let request_ident = RequestIdent::new(packet.request_id, ev.connection.id());

        if let Ok(generator_id) = ctx
            .generators
            .lock()
            .await
            .register_generator(&packet.name, packet.factory_params())
        {
            ev.connection
                .send_packet(&packets::ConfirmGeneratorAddition::new(
                    request_ident.request_id,
                    generator_id,
                ))
                .await
                .unwrap();
        }
    }
}

async fn handle_generated_chunk(mut provider: SingleEventProvider<Context, ChunkFinished>) {
    while let Some((ctx, event)) = provider.next().await {
        match event.result {
            GenerationResult::Success(ident, chunk) => {
                let packet = packets::ReplyChunk {
                    request_id: ident.into(),
                    chunk,
                };

                if let Some(conn) = ctx.networker.connection(ident.into()).await {
                    conn.send_packet(&packet).await.unwrap();
                }
            }
            GenerationResult::Failure(ident, error) => {
                log::error!("Request {ident:?} failed: {error}");
                // let net_error = ProtocolErrorKind::ChunkGenerationFailure { generator_id: , request_id: () };
                // let packet = ProtocolError::gentle()
            }
        }
    }
}
