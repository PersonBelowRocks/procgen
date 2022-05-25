use ecs::prelude::*;

use crate::runtime::{
    components::{Generator, GeneratorName},
    events::{FinishedGenerateChunkEvent, RequestGenerateChunkEvent},
    resources::{ChunkGenerationOutput, GenerationPool},
};

pub fn generate_chunks(
    mut reader: EventReader<RequestGenerateChunkEvent>,
    pool: Res<GenerationPool>,
    q_generators: Query<(&GeneratorName, &Generator)>,
) {
    for request in reader.iter() {
        // Find the generator with the requested name.
        let (_, generator) = q_generators
            .iter()
            .find(|&(name, _)| request.name == *name)
            .expect("invalid generator name");

        // Submit this request to our generation pool, we'll collect it in another system.
        pool.submit(request.args, request.request, generator.clone())
    }
}

pub fn collect_chunks(
    mut writer: EventWriter<FinishedGenerateChunkEvent>,
    outputs: Res<ChunkGenerationOutput>,
) {
    // TODO: it might be nicer to not have this system and instead just pull the chunks out directly where they're needed from the ChunkGenerationOutput.
    writer.send_batch(outputs.iter_poll())
}
