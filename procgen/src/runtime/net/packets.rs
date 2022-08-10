use common::packets::*;

use procgen_common::packets::GenerateRegion;

use super::DynPacket;

pub fn parse_dyn(buf: &PacketBuffer) -> anyhow::Result<DynPacket> {
    let id = buf.id();

    match id {
        ProtocolError::ID => Ok(Box::new(buf.to_packet::<ProtocolError>()?)),
        GenerateRegion::ID => Ok(Box::new(buf.to_packet::<GenerateRegion>()?)),
        GenerateBrush::ID => Ok(Box::new(buf.to_packet::<GenerateBrush>()?)),
        VoxelData::ID => Ok(Box::new(buf.to_packet::<VoxelData>()?)),
        FinishRequest::ID => Ok(Box::new(buf.to_packet::<FinishRequest>()?)),

        _ => Err(anyhow::anyhow!("invalid packet ID")),
    }
}
