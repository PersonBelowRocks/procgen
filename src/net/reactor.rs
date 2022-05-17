use super::{
    connection::Connection,
    protocol::{DownstreamSuite, UpstreamSuite},
    server::AsyncStream,
};

pub(super) struct PacketReactor {
    generator_manager: (),
}

pub(super) struct Context<'a, S: AsyncStream> {
    connection: &'a mut Connection<S>,
}

impl<'a, S: AsyncStream> Context<'a, S> {
    pub fn new(connection: &'a mut Connection<S>) -> Self {
        Self { connection }
    }

    pub async fn send_packet(&mut self, packet: DownstreamSuite) -> anyhow::Result<()> {
        todo!()
    }
}

impl PacketReactor {
    pub fn new(generator_manager: ()) -> Self {
        Self { generator_manager }
    }

    pub async fn react<S: AsyncStream>(
        &self,
        packet: UpstreamSuite,
        ctx: Context<'_, S>,
    ) -> anyhow::Result<()> {
        todo!()
    }
}
