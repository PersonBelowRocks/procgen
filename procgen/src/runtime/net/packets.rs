use common::packets::*;

use procgen_common::packets::GenerateRegion;

use super::DynPacket;

pub fn parse_dyn(buf: &PacketBuffer) -> anyhow::Result<DynPacket> {
    let id = buf.id();

    match id {
        GenerateChunk::ID => Ok(Box::new(buf.to_packet::<GenerateChunk>()?)),
        ReplyChunk::ID => Ok(Box::new(buf.to_packet::<ReplyChunk>()?)),
        AddGenerator::ID => Ok(Box::new(buf.to_packet::<AddGenerator>()?)),
        ConfirmGeneratorAddition::ID => Ok(Box::new(buf.to_packet::<ConfirmGeneratorAddition>()?)),
        ProtocolError::ID => Ok(Box::new(buf.to_packet::<ProtocolError>()?)),

        GenerateRegion::ID => Ok(Box::new(buf.to_packet::<GenerateRegion>()?)),

        _ => Err(anyhow::anyhow!("invalid packet ID")),
    }
}
