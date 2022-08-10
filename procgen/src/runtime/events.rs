use std::sync::Arc;

use procgen_common::{
    packets::{self, DowncastPacket, Packet, ProtocolErrorKind},
    Bounded, Boundness, RequestId, Unbounded, VoxelVolume,
};
use tokio::sync::Mutex;
use vol::Volume;

use crate::generation::{GenBrushRequest, GenRegionRequest, GeneratorManager};

use super::{
    dispatcher::{self, BcstEventProvider, Dispatcher, DispatcherContext},
    net::{Connection, Networker},
};

pub async fn defaults(dispatcher: &Dispatcher<Context>, generator_manager: &GeneratorManager) {
    dispatcher.register_bcst(handle_incoming_packet).await;
    dispatcher.register_bcst(handle_generate_region).await;
    dispatcher.register_bcst(handle_generate_brush).await;

    dispatcher
        .register_single(handle_finished_generating_region)
        .await;
    dispatcher
        .register_single(handle_finished_generating_brush)
        .await;

    let provider = dispatcher
        .single_handler::<GenerateBrushEvent>()
        .await
        .unwrap();
    generator_manager.generate_brush_listener(provider).await;

    let provider = dispatcher
        .single_handler::<GenerateRegionEvent>()
        .await
        .unwrap();
    generator_manager.generate_region_listener(provider).await;
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

    fn broadcast_event_blocking<E: dispatcher::BroadcastedEvent>(&self, event: E) -> bool {
        self.dispatcher
            .broadcast_event_blocking(self.clone(), event)
    }

    fn fire_event_blocking<E: dispatcher::SingleEvent>(&self, event: E) -> bool {
        self.dispatcher.fire_event_blocking(self.clone(), event)
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

pub struct GenerateRegionEvent {
    pub request_id: RequestId,
    pub request: GenRegionRequest,
    pub connection: Connection,
}

pub struct FinishedGeneratingRegionEvent {
    pub request_id: RequestId,
    pub volume: anyhow::Result<VoxelVolume<Bounded>>,
    pub connection: Connection,
    pub generator_name: String,
}

pub struct GenerateBrushEvent {
    pub request_id: RequestId,
    pub request: GenBrushRequest,
    pub connection: Connection,
}

pub struct FinishedGeneratingBrushEvent {
    pub request_id: RequestId,
    pub volume: anyhow::Result<VoxelVolume<Unbounded>>,
    pub connection: Connection,
    pub pos: na::Vector3<i64>,
    pub generator_name: String,
}

type Prov<E> = BcstEventProvider<Context, E>;
type PRecv<P> = Prov<ReceivedPacket<P>>;

async fn handle_incoming_packet(ctx: Arc<Context>, event: IncomingPacket) {
    if let Some(packet) = event.packet.downcast_ref::<packets::GenerateRegion>() {
        println!("received GenerateRegion!");

        let packet = ReceivedPacket {
            connection: event.connection.clone(),
            packet: Arc::new(packet.clone()),
        };

        ctx.broadcast_event(packet).await;
    }
}

async fn handle_generate_region(ctx: Arc<Context>, event: ReceivedPacket<packets::GenerateRegion>) {
    let packet = event.packet;
    log::info!("Received request to generate region: {packet:?}");

    let next_event = GenerateRegionEvent {
        request_id: packet.request_id,
        request: GenRegionRequest {
            region: packet.bounds.clone().into(),
            parameters: packet.params.clone(),
        },
        connection: event.connection,
    };

    ctx.fire_event(next_event).await;
}

async fn handle_generate_brush(ctx: Arc<Context>, event: ReceivedPacket<packets::GenerateBrush>) {
    let packet = event.packet;
    log::info!("Received request to generate brush: {packet:?}");

    let next_event = GenerateBrushEvent {
        request_id: packet.request_id,
        request: GenBrushRequest {
            pos: packet.pos,
            parameters: packet.params.clone(),
        },
        connection: event.connection,
    };

    ctx.fire_event(next_event).await;
}

async fn send_voxel_data<P: Boundness>(
    connection: &Connection,
    request_id: RequestId,
    vol: VoxelVolume<P>,
) -> anyhow::Result<()> {
    for chunk in vol.into_chunks() {
        connection
            .send_packet(&packets::VoxelData {
                request_id,
                data: chunk,
            })
            .await?
    }

    Ok(())
}

async fn handle_finished_generating_region(
    _ctx: Arc<Context>,
    event: FinishedGeneratingRegionEvent,
) {
    log::info!(
        "Finished generating region. Request ID: {}, region: {:?}",
        event.request_id,
        (&event.volume).as_ref().map(|v| v.bounding_box())
    );

    match event.volume {
        Ok(vol) => {
            send_voxel_data(&event.connection, event.request_id, vol)
                .await
                .unwrap();

            event
                .connection
                .send_packet(&packets::FinishRequest {
                    request_id: event.request_id,
                })
                .await
                .unwrap();
        }
        Err(error) => {
            event
                .connection
                .gentle_error(ProtocolErrorKind::GenerationError {
                    generator_name: event.generator_name,
                    request_id: event.request_id,
                    details: error.to_string(),
                })
                .await
                .unwrap();
        }
    }
}

async fn handle_finished_generating_brush(_ctx: Arc<Context>, event: FinishedGeneratingBrushEvent) {
    log::info!(
        "Finished generating brush. Request ID: {}, pos: {}",
        event.request_id,
        event.pos
    );

    match event.volume {
        Ok(vol) => {
            send_voxel_data(&event.connection, event.request_id, vol)
                .await
                .unwrap();

            event
                .connection
                .send_packet(&packets::FinishRequest {
                    request_id: event.request_id,
                })
                .await
                .unwrap();
        }
        Err(error) => {
            event
                .connection
                .gentle_error(ProtocolErrorKind::GenerationError {
                    generator_name: event.generator_name,
                    request_id: event.request_id,
                    details: error.to_string(),
                })
                .await
                .unwrap();
        }
    }
}
