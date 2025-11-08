use bevy::prelude;
use derive_where::derive_where;

#[derive_where(Debug; spru::common::Seed<Client::Common>)]
pub struct Init<Client: super::ClientSSS> {
    pub seed: spru::common::Seed<Client::Common>,
}

impl<Client: super::ClientSSS> prelude::Command for Init<Client> {
    fn apply(self, world: &mut bevy::ecs::world::World) {
        let Self { seed } = self;

        let mut command_queue = bevy::ecs::world::CommandQueue::default();

        super::system::init::<Client>(seed, world, &mut command_queue);

        command_queue.apply(world);
    }
}
