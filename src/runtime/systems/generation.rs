use ecs::prelude::*;

use crate::runtime::{
    components::{Generator, GeneratorName},
    events::{FinishedGenerateChunkEvent, RequestGenerateChunkEvent},
};

pub fn generate_chunk(
    mut reader: EventReader<RequestGenerateChunkEvent>,
    _writer: EventWriter<FinishedGenerateChunkEvent>,
    q_generators: Query<(&GeneratorName, &Generator)>,
) {
    for request in reader.iter() {
        // Find the generator with the requested name.
        let (_, _generator) = q_generators
            .iter()
            .find(|&(name, _)| request.name == *name)
            .expect("invalid generator name");

        // let chunk = process_request(&request.args, generator);
    }
}
