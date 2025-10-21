use bevy::prelude;
use derive_where::derive_where;

#[derive_where(Debug; spru::client::init::Arg<Client::Common>)]
pub struct Init<Client: super::ClientSSS> {
    pub init: spru::client::init::Arg<Client::Common>,
}

impl<Client: super::ClientSSS> prelude::Command for Init<Client> {
    fn apply(self, world: &mut bevy::ecs::world::World) {
        let Self {
            init,
        } = self;

        let mut command_queue = bevy::ecs::world::CommandQueue::default();

        super::system::init::<Client>(init, world, &mut command_queue);

        command_queue.apply(world);
    }
}

// #[derive_where(Debug; Client::Interaction)]
// pub struct StageInteraction<Client: super::ClientSSS> {
//     pub interaction: Client::Interaction,
//     pub game_id: common::component::GameId,
//     pub client_id: super::component::ClientId,
// }

// impl<Client: super::ClientSSS> prelude::Command<prelude::Result<spru::transaction::Pending>> for StageInteraction<Client> {
//     fn apply(self, world: &mut bevy::ecs::world::World) 
//         -> prelude::Result<spru::transaction::Pending> 
//     {
//         use bevy::ecs::system::RunSystemOnce as _;

//         let Self {
//             interaction,
//             game_id,
//             client_id,
//         } = self;

//         let client_entity = world.run_system_once_with(super::system::find_client::<Client>, (game_id, client_id.clone()))
//             .expect("System must be valid") 
//             .ok_or("Client not found")?;

//         let mut bundle = world.entity_mut(client_entity)
//             .take::<(
//                 super::component::Runner<Client>,
//                 super::component::EntityMap,
//                 super::component::ToServer<Client>,
//                 common::component::GameOutcome<Client::Common>,
//             )>()
//             .ok_or("Client missing components")?;

//         let (
//             runner,
//             entity_map,
//             to_server,
//             game_outcome,
//         ) = &mut bundle;

//         let mut lookup = super::BevyLookup::new(world, entity_map.inner_mut(), client_id.0);
        
//         let result = super::system::stage_interaction(&mut lookup, runner, to_server, game_outcome, interaction);

//         world.entity_mut(client_entity)
//             .insert(bundle);

//         Ok(result?)
//     }
// }

// #[derive_where(Debug; )]
// pub struct ApplyInteractions<Client: super::ClientSSS> {
//     pub game_id: common::component::GameId,
//     pub client_id: super::component::ClientId,
//     pub pending_transaction: Option<spru::transaction::Pending>,
//     pub client: PhantomData<fn() -> Client>,
// }

// impl<Client: super::ClientSSS> prelude::Command<prelude::Result<()>> for ApplyInteractions<Client> {
//     fn apply(self, world: &mut bevy::ecs::world::World) -> prelude::Result<()> {
//         use bevy::ecs::system::RunSystemOnce as _;

//         let Self {
//             pending_transaction,
//             game_id,
//             client_id,
//             client: _client,
//         } = self;

//         let client_entity = world.run_system_once_with(super::system::find_client::<Client>, (game_id, client_id.clone()))
//             .expect("System must be valid") 
//             .ok_or("Client not found")?;

//         let mut bundle = world.entity_mut(client_entity)
//             .take::<(
//                 super::component::Runner<Client>,
//                 super::component::EntityMap,
//                 super::component::ToServer<Client>,
//                 common::component::GameOutcome<Client::Common>,
//             )>()
//             .ok_or("Client missing components")?;

//         let (
//             runner,
//             entity_map,
//             to_server,
//             game_outcome,
//         ) = &mut bundle;

//         let mut lookup = super::BevyLookup::new(world, entity_map.inner_mut(), client_id.0);

//         let result = super::system::apply_interactions(&mut lookup, runner, to_server, game_outcome, pending_transaction);

//         world.entity_mut(client_entity)
//             .insert(bundle);
        
//         Ok(result?)
//     }
// }

// #[derive_where(Debug; )]
// pub struct RevertInteractions<Client: super::ClientSSS> {
//     pub game_id: common::component::GameId,
//     pub client_id: super::component::ClientId,
//     pub pending_transaction: Option<spru::transaction::Pending>,
//     pub client: PhantomData<fn() -> Client>,
// }

// impl<Client: super::ClientSSS> prelude::Command<prelude::Result<()>> for RevertInteractions<Client> {
//     fn apply(self, world: &mut bevy::ecs::world::World) -> prelude::Result<()> {
//         use bevy::ecs::system::RunSystemOnce as _;

//         let Self {
//             pending_transaction,
//             game_id,
//             client_id,
//             client: _client,
//         } = self;

//         let client_entity = world.run_system_once_with(super::system::find_client::<Client>, (game_id, client_id.clone()))
//             .expect("System must be valid") 
//             .ok_or("Client not found")?;

//         let mut bundle = world.entity_mut(client_entity)
//             .take::<(
//                 super::component::Runner<Client>,
//                 super::component::EntityMap,
//                 super::component::ToServer<Client>,
//                 common::component::GameOutcome<Client::Common>,
//             )>()
//             .ok_or("Client missing components")?;

//         let (
//             runner,
//             entity_map,
//             to_server,
//             game_outcome,
//         ) = &mut bundle;

//         let mut lookup = super::BevyLookup::new(world, entity_map.inner_mut(), client_id.0);

//         let result = super::system::revert_interactions(&mut lookup, runner, to_server, game_outcome, pending_transaction);
//         world.trigger_ref(event);

//         world.entity_mut(client_entity)
//             .insert(bundle);

//         Ok(result?)
//     }
// }
