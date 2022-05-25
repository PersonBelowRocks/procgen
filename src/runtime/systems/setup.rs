use super::super::GeneratorManager;
use ecs::prelude::*;

pub fn setup(mut cmds: Commands) {
    cmds.insert_resource(GeneratorManager::new());
}
