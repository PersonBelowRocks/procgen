use std::{
    io::{Read, Write},
    net::{SocketAddrV4, TcpStream},
    time::Duration,
};

use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
use volume::Volume;

use crate::{
    block::BlockId,
    chunk::{Chunk, Spaces},
    generation::{ChunkGenerator, FactoryParameters, GenerationArgs, GeneratorFactory},
    runtime::server::{Server, ServerParams},
};

use super::net::{
    packets::{self, GenerateChunk, Packet, ReplyChunk},
    Header, Networker, Params,
};

struct MockClient {
    stream: TcpStream,
}

impl MockClient {
    fn new(addr: SocketAddrV4) -> Self {
        Self {
            stream: TcpStream::connect(addr).unwrap(),
        }
    }

    fn send_packet<P: Packet>(&mut self, packet: &P) -> anyhow::Result<()> {
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

    fn read_packet<P: Packet>(&mut self) -> anyhow::Result<P> {
        let header = Header::sync_read(&mut self.stream)?;
        let mut compressed_buf = vec![0u8; header.compressed_len as usize];

        self.stream.read_exact(&mut compressed_buf)?;

        let decompressed_buf = {
            let mut buf = Vec::<u8>::with_capacity(header.decompressed_len as usize);
            let mut decompressor = ZlibDecoder::new(&compressed_buf[..]);

            decompressor.read_to_end(&mut buf)?;
            buf
        };

        let packet = bincode::deserialize::<P>(&decompressed_buf[2..])?;
        Ok(packet)
    }
}

struct MockGenFactory;

impl GeneratorFactory for MockGenFactory {
    type Generator = MockGenerator;

    fn create(&self, params: FactoryParameters<'_>) -> Self::Generator {
        MockGenerator {
            min_height: params.min_height,
            max_height: params.max_height,
            default_id: params.default,
        }
    }
}

struct MockGenerator {
    min_height: i32,
    max_height: i32,
    default_id: BlockId,
}

impl ChunkGenerator for MockGenerator {
    const NAME: &'static str = "MOCK_GENERATOR";

    type Factory = MockGenFactory;

    fn generate(&self, args: &GenerationArgs) -> anyhow::Result<Chunk> {
        let mut chunk = Chunk::new(self.default_id, args.pos, self.min_height, self.max_height);

        for x in 0..16 {
            for z in 0..16 {
                chunk.set(Spaces::Cs([x, self.min_height, z]), 80.into());
            }
        }

        Ok(chunk)
    }

    fn factory() -> Self::Factory {
        MockGenFactory
    }
}

// TODO: this test is great and all, but we should also have some tests for more abnormal behaviour, like malformed packets
#[tokio::test]
async fn networker_recv() {
    let params = Params {
        addr: "0.0.0.0:33445".parse().unwrap(),
        compression: Compression::best(),
    };

    let mut networker = Networker::new(params);

    networker.run().await.unwrap();

    let mut client = MockClient::new("127.0.0.1:33445".parse::<SocketAddrV4>().unwrap());

    let packet = packets::AddGenerator {
        request_id: 42.into(),
        name: "hello!!!".to_string(),
        min_height: -64,
        max_height: 320,
        default_id: 0.into(),
    };

    client.send_packet(&packet).unwrap();

    // we need to sleep a lil so the packet has time to arrive
    tokio::time::sleep(Duration::from_millis(500)).await;

    for incoming in networker.incoming().await {
        let packet = incoming
            .1
            .as_ref()
            .unwrap()
            .downcast_ref::<packets::AddGenerator>()
            .unwrap();
        assert_eq!(packet.name, "hello!!!");
        assert_eq!(packet.request_id, 42.into());
    }
}

#[tokio::test]
async fn networker_send() {
    let params = Params {
        addr: "0.0.0.0:33446".parse().unwrap(),
        compression: Compression::best(),
    };

    let mut networker = Networker::new(params);
    networker.run().await.unwrap();

    let mut client = MockClient::new("127.0.0.1:33446".parse::<SocketAddrV4>().unwrap());

    let generate_chunk_packet = GenerateChunk {
        request_id: 560.into(),
        generator_id: 4.into(),
        pos: na::vector![-6, 2],
    };

    client.send_packet(&generate_chunk_packet).unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;

    let mut chunk = Chunk::new(77.into(), na::vector![-6, 2], -64, 320);
    chunk.set(Spaces::Cs([10i32, 120, 8]), 80.into());
    chunk.set(Spaces::Cs([6i32, -20, 9]), 92.into());

    for (conn, raw_packet) in networker.incoming().await {
        let packet = raw_packet
            .as_ref()
            .unwrap()
            .downcast_ref::<GenerateChunk>()
            .unwrap();

        let chunk_packet = ReplyChunk {
            request_id: packet.request_id,
            chunk: chunk.clone(),
        };

        conn.send_packet(&chunk_packet).await.unwrap();
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    let received_packet = client.read_packet::<ReplyChunk>().unwrap();

    assert_eq!(received_packet.request_id, 560.into());
    assert_eq!(received_packet.chunk, chunk);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn end_to_end_server_test() {
    let params = ServerParams {
        addr: "0.0.0.0:33443".parse().unwrap(),
        compression: Compression::best(),
        coarsening: 100,
    };

    let mut server = Server::new(params);

    server.add_generator::<MockGenerator>().await;

    server.run().await;

    let mut client = MockClient::new("127.0.0.1:33443".parse().unwrap());

    client
        .send_packet(&packets::AddGenerator {
            request_id: 500.into(),
            name: MockGenerator::NAME.to_string(),
            min_height: -64,
            max_height: 320,
            default_id: 21.into(),
        })
        .unwrap();

    tokio::time::sleep(Duration::from_millis(250)).await;

    let generator_id = {
        let packet = client
            .read_packet::<packets::ConfirmGeneratorAddition>()
            .unwrap();
        assert_eq!(packet.request_id, 500.into());
        packet.generator_id
    };

    client
        .send_packet(&packets::GenerateChunk {
            request_id: 420.into(),
            generator_id,
            pos: na::vector![6i32, 4],
        })
        .unwrap();

    tokio::time::sleep(Duration::from_millis(250)).await;

    let packet = client.read_packet::<packets::ReplyChunk>().unwrap();
    assert_eq!(packet.request_id, 420.into());

    for x in 0..16 {
        for z in 0..16 {
            assert_eq!(
                packet.chunk.get(Spaces::Cs([x, -64, z])),
                Some(&BlockId::new(80))
            );
        }
    }
}
