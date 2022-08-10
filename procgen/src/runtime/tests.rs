use std::{
    io::{Read, Write},
    net::{SocketAddrV4, TcpStream},
    sync::Arc,
    time::Duration,
};

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use procgen_common::{Bounded, Chunk, Parameters, Positioned, VoxelSlot, VoxelVolume};
use volume::Volume;

use crate::{
    generation::{RegionGenerator, RegionGeneratorFactory},
    runtime::server::{Server, ServerParams},
};

use super::net::{Header, Networker, Params};

use common::packets::{self, *};

mod mock {
    use crate::runtime::dispatcher::BroadcastedEvent;
    use crate::runtime::dispatcher::Dispatcher;
    use crate::runtime::dispatcher::DispatcherContext;
    use crate::runtime::dispatcher::SingleEvent;
    use crate::runtime::net::packets::parse_dyn;

    use super::*;

    #[derive(Clone)]
    pub struct Context(pub Arc<Dispatcher<Self>>);

    #[async_trait::async_trait]
    impl DispatcherContext for Context {
        async fn broadcast_event<E: BroadcastedEvent>(&self, event: E) -> bool {
            self.0.broadcast_event(self.clone(), event).await
        }

        async fn fire_event<E: SingleEvent>(&self, event: E) -> bool {
            self.0.fire_event(self.clone(), event).await
        }

        fn broadcast_event_blocking<E: BroadcastedEvent>(&self, _event: E) -> bool {
            unreachable!()
        }

        fn fire_event_blocking<E: SingleEvent>(&self, _event: E) -> bool {
            unreachable!()
        }
    }

    #[derive(te::Error, Debug)]
    #[error("The next packet is of a different type")]
    pub struct IncorrectPacketType;

    pub struct MockClient {
        packet: Option<Box<dyn DowncastPacket>>,
        stream: TcpStream,
    }

    impl MockClient {
        pub fn new(addr: SocketAddrV4) -> Self {
            Self {
                packet: None,
                stream: TcpStream::connect(addr).unwrap(),
            }
        }

        pub fn send_packet<P: Packet>(&mut self, packet: &P) -> anyhow::Result<()> {
            let mut buf = P::ID.to_be_bytes().to_vec();
            buf.extend(bincode::serialize(packet)?);

            let decompressed_len = buf.len();

            let compressed_buf = {
                let mut compressed_buf = Vec::<u8>::new();
                let mut compressor = ZlibEncoder::new(&mut compressed_buf, Compression::best());
                compressor.write_all(&buf)?;
                compressor.finish()?;

                compressed_buf
            };

            let header = Header::new(compressed_buf.len() as u32, decompressed_len as u32);

            header.sync_write(&mut self.stream)?;
            self.stream.write_all(&compressed_buf)?;

            self.stream.flush()?;

            Ok(())
        }

        pub fn read_packet<P: Packet + DowncastPacket>(&mut self) -> anyhow::Result<P> {
            if self.packet.is_some() {
                let packet = self.packet.take().unwrap();

                match packet.is::<P>() {
                    true => Ok(*packet.downcast::<P>().unwrap()),
                    false => {
                        self.packet = Some(packet);
                        Err(IncorrectPacketType.into())
                    }
                }
            } else {
                let packet = self.read_tcp_packet()?;

                match packet.is::<P>() {
                    true => Ok(*packet.downcast::<P>().unwrap()),
                    false => {
                        self.packet = Some(packet);
                        Err(IncorrectPacketType.into())
                    }
                }
            }
        }

        fn read_tcp_packet(&mut self) -> anyhow::Result<Box<dyn DowncastPacket>> {
            let header = Header::sync_read(&mut self.stream)?;
            let mut compressed_buf = vec![0u8; header.compressed_len as usize];

            self.stream.read_exact(&mut compressed_buf)?;

            let decompressed_buf = {
                let mut decompressor = ZlibDecoder::new(&compressed_buf[..]);
                PacketBuffer::from_reader(&mut decompressor)?
            };

            parse_dyn(&decompressed_buf)
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn networker_recv() {
    let params = Params {
        addr: "0.0.0.0:33445".parse().unwrap(),
        compression: Compression::best(),
    };

    let mut networker = Networker::new(params);

    networker.run().await.unwrap();

    let mut client = mock::MockClient::new("127.0.0.1:33445".parse::<SocketAddrV4>().unwrap());

    let packet = packets::GenerateBrush {
        request_id: 42.into(),
        pos: [10, 11, 12].into(),
        params: Parameters {
            generator_name: "example".into(),
        },
    };

    client.send_packet(&packet).unwrap();

    // we need to sleep a lil so the packet has time to arrive
    tokio::time::sleep(Duration::from_millis(75)).await;

    for incoming in networker.incoming().await {
        let packet = incoming
            .1
            .as_ref()
            .unwrap()
            .downcast_ref::<packets::GenerateBrush>()
            .unwrap();

        assert_eq!(packet.request_id, 42.into());
        assert_eq!(packet.pos, na::vector![10, 11, 12i64]);
        assert_eq!(packet.params.generator_name(), "example");
    }

    networker.stop().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn networker_send() {
    let params = Params {
        addr: "0.0.0.0:33446".parse().unwrap(),
        compression: Compression::best(),
    };

    let mut networker = Networker::new(params);
    networker.run().await.unwrap();

    let mut client = mock::MockClient::new("127.0.0.1:33446".parse::<SocketAddrV4>().unwrap());

    let generate_chunk_packet = GenerateRegion {
        request_id: 560.into(),
        bounds: na::vector![2, 2, 2]..na::vector![10, 10, 10],
        params: Parameters {
            generator_name: "example_region".into(),
        },
    };

    client.send_packet(&generate_chunk_packet).unwrap();

    // we need to box this fella otherwise we overflow the stack
    let mut vol = Chunk::<Positioned>::new([10, 11, 12].into());

    vol.set([9, 7, 5].into(), 493.into());

    tokio::time::sleep(Duration::from_millis(75)).await;

    for (conn, raw_packet) in networker.incoming().await {
        let packet = raw_packet
            .as_ref()
            .unwrap()
            .downcast_ref::<GenerateRegion>()
            .unwrap();

        let data_packet = VoxelData {
            request_id: packet.request_id,
            data: vol.clone(),
        };

        conn.send_packet(&data_packet).await.unwrap();
    }

    tokio::time::sleep(Duration::from_millis(50)).await;

    let received_packet = client.read_packet::<VoxelData>().unwrap();

    assert_eq!(received_packet.request_id, 560.into());
    assert_eq!(
        received_packet.data.get([9, 10, 5].into()),
        vol.get([9, 10, 5].into())
    );

    networker.stop().await.unwrap();
}

// TODO: more dispatcher tests!
#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn dispatcher_broadcast() {
    use crate::runtime::dispatcher::*;

    #[derive(Clone)]
    struct SomeEvent(i32);

    #[derive(Clone)]
    struct OtherEvent(i32);

    let dispatcher = Arc::new(Dispatcher::<mock::Context>::new(20));

    let mut handle = dispatcher.broadcast_handler::<SomeEvent>().await;

    let j = tokio::spawn(async move {
        while let Some((ctx, event)) = handle.next().await {
            assert_eq!(event.0, 42);

            ctx.broadcast_event(OtherEvent(420)).await;
        }
    });

    let mut handle = dispatcher.broadcast_handler::<OtherEvent>().await;
    let k = tokio::spawn(async move {
        while let Some((_ctx, event)) = handle.next().await {
            assert_eq!(event.0, 420);
        }
    });

    dispatcher
        .broadcast_event(mock::Context(dispatcher.clone()), SomeEvent(42))
        .await;
    drop(dispatcher);

    assert!(j.await.is_ok());
    assert!(k.await.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn dispatcher_single() {
    use crate::runtime::dispatcher::*;

    #[derive(Clone)]
    struct SomeEvent(i32);

    #[derive(Clone)]
    struct OtherEvent(i32);

    let dispatcher = Arc::new(Dispatcher::<mock::Context>::new(20));

    let mut handle = dispatcher.single_handler::<SomeEvent>().await.unwrap();

    let j = tokio::spawn(async move {
        while let Some((ctx, event)) = handle.next().await {
            assert_eq!(event.0, 42);

            ctx.fire_event(OtherEvent(420)).await;
        }
    });

    let mut handle = dispatcher.single_handler::<OtherEvent>().await.unwrap();
    let k = tokio::spawn(async move {
        while let Some((_ctx, event)) = handle.next().await {
            assert_eq!(event.0, 420);
        }
    });

    dispatcher
        .fire_event(mock::Context(dispatcher.clone()), SomeEvent(42))
        .await;
    drop(dispatcher);

    assert!(j.await.is_ok());
    assert!(k.await.is_ok());
}

// TODO: end to end test(s) for the server, triggering various events and generating some stuff.

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn end_to_end_server_test() {
    env_logger::init();

    struct MockGenerator;

    #[derive(Debug, te::Error)]
    #[error("Example error message (generator)")]
    struct MockGeneratorError;

    impl RegionGenerator for MockGenerator {
        type Error = MockGeneratorError;

        fn generate(
            &self,
            vol: &mut VoxelVolume<Bounded>,
            _ctx: crate::generation::GenerationContext,
        ) -> Result<(), Self::Error> {
            let min = vol.bounding_box().min();
            let max = vol.bounding_box().max();

            for x in min.x..max.x {
                for z in min.z..max.z {
                    vol.set([x, min.y, z].into(), 100.into());
                }
            }

            Ok(())
        }
    }

    struct MockFactory;

    #[derive(Debug, te::Error)]
    #[error("Example error message (factory)")]
    struct MockFactoryError;

    impl RegionGeneratorFactory for MockFactory {
        type Error = MockFactoryError;
        type Generator = MockGenerator;

        fn new_generator(&self, _params: &Parameters) -> Result<Self::Generator, Self::Error> {
            Ok(MockGenerator)
        }

        fn name(&self) -> String {
            "MOCK_FACTORY".into()
        }
    }

    // Initialize and set everything up
    let mut server = Server::new(ServerParams {
        addr: "0.0.0.0:28989".parse().unwrap(),
        compression: Compression::best(),
        coarsening: 100,
    })
    .await;

    server.add_region_generator(MockFactory).await;
    server.run().await;

    let mut client = mock::MockClient::new("127.0.0.1:28989".parse().unwrap());

    let region_bounds = na::vector![60, 60, 60]..na::vector![100, 100, 100];
    client
        .send_packet(&packets::GenerateRegion {
            request_id: 505.into(),
            bounds: region_bounds.clone(),
            params: Parameters {
                generator_name: "MOCK_FACTORY".into(),
            },
        })
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let mut received_volume = VoxelVolume::<Bounded>::new(region_bounds.clone().into());

    loop {
        let packet = client.read_packet::<packets::VoxelData>();

        match packet {
            Ok(voxel_packet) => received_volume.add_chunk(voxel_packet.data),
            Err(error) => {
                if error.is::<mock::IncorrectPacketType>() {
                    break;
                }
            }
        }
    }

    let expected_voxels = (region_bounds.start - region_bounds.end).xz().product();
    let mut found_voxels = 0;

    let min = received_volume.bounding_box().min();
    let max = received_volume.bounding_box().max();

    for x in min.x..max.x {
        for z in min.z..max.z {
            let slot = received_volume.get([x, min.y, z].into());

            assert_eq!(slot, VoxelSlot::Occupied(100.into()));
            found_voxels += 1;
        }
    }

    assert_eq!(found_voxels, expected_voxels);

    server.stop().await.unwrap();

    // todo!()
}
