use crate::runtime::resources::GenerationPool;

use ecs::prelude::*;

pub fn setup(mut cmds: Commands) {
    let (tx, rx) = GenerationPool::new();

    cmds.insert_resource(tx);
    cmds.insert_resource(rx);
}
