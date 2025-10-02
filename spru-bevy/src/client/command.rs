use bevy::prelude;
use derive_where::derive_where;

#[derive_where(Debug; spru::client::init::Arg<Client::Common>)]
pub struct SpawnClient<Client: super::ClientSSS> {
    pub init: spru::client::init::Arg<Client::Common>,
    pub game_id: crate::common::component::GameId,
}

impl<Client: super::ClientSSS> prelude::Command<prelude::Result<prelude::Entity>> for SpawnClient<Client> {
    fn apply(self, world: &mut bevy::ecs::world::World) -> prelude::Result<prelude::Entity> {
        let runner = super::component::Runner::<Client>::new(world, self.init)?;
        let player_id = runner.inner().client.local_player_id();
        Ok(world.spawn((
            runner,
            self.game_id,
            crate::client::component::ClientId(player_id),
        )).id())
    }
}