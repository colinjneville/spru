use bevy::prelude;
use derive_where::derive_where;

#[derive_where(Debug; GameInit, Server::PlayerInit, Server::Reaction)]
pub struct Init<Server: super::ServerSSS, GameInit> {
    pub game_init: GameInit,
    pub player_init: Server::PlayerInit,
    pub reaction: Server::Reaction,
}

impl<Server, GameInit> prelude::Command<()> for Init<Server, GameInit>
where
    Server: super::ServerSSS,
    GameInit: spru::game::Init<State = Server::State, Action = Server::Action, Root = Server::Root>
        + Send
        + Sync
        + 'static,
{
    fn apply(self, world: &mut bevy::ecs::world::World) {
        let Self {
            game_init,
            player_init,
            reaction,
        } = self;

        super::system::init::<Server, GameInit>(
            game_init,
            player_init,
            reaction,
            &mut world.commands(),
        );
    }
}
