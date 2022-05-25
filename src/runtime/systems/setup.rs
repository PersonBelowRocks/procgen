use crate::runtime::resources::GenerationPool;

use super::super::GeneratorManager;
use ecs::prelude::*;

pub fn setup(mut cmds: Commands) {
    cmds.insert_resource(GeneratorManager::new());

    let (tx, rx) = GenerationPool::new();

    cmds.insert_resource(tx);
    cmds.insert_resource(rx);
}
