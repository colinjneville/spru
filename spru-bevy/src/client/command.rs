use bevy::prelude;

#[derive(Debug)]
pub struct SpawnClient<Client: super::ClientSSS, State> {
    pub init: spru::client::init::Arg<State, Client::Action, Client::Root>,
    pub game_id: crate::common::component::GameId,
}

impl<Client, State> prelude::Command<prelude::Result<prelude::Entity>> for SpawnClient<Client, State> 
where 
    Client: super::ClientSSS,
    State: for<'l> spru::State<super::BevyLookup<'l>, Repr: TryFrom<spru::state::Index>> + 'static,
{
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