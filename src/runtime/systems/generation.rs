use ecs::prelude::*;

use crate::runtime::{
    components::{Generator, GeneratorName},
    events::{FinishedGenerateChunkEvent, RequestGenerateChunkEvent},
};

pub fn generate_chunk(
    mut reader: EventReader<RequestGenerateChunkEvent>,
    mut writer: EventWriter<FinishedGenerateChunkEvent>,
    generators: Query<(&GeneratorName, &Generator)>,
) {
    for request in reader.iter() {
        todo!()
    }
}
