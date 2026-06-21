use std::collections::HashMap;

use bevy::prelude;

pub(crate) struct Client;

impl prelude::Plugin for Client {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .add_plugins((
                spru_bevy::client::Plugin::<crate::Client>::default(),
            ))
            .init_resource::<ClientMap>()
            .add_observer(|
                    client_add: prelude::On<prelude::Add, spru_bevy::client::component::Runner<crate::Client>>,
                    mut commands: prelude::Commands,
                    mut client_map: prelude::ResMut<ClientMap>,
                    q_client_id: prelude::Query<(
                        &spru_bevy::common::component::GameId,
                        &spru_bevy::client::component::ClientId,
                    )>,
                | {
                    let (game_id, client_id, ) = q_client_id.get(client_add.entity)
                        .expect("Entity does not have a game and client id");
                    client_map.insert(**game_id, **client_id, client_add.entity);
                    
                    commands.entity(client_add.entity)
                        .insert_if_new((
                            crate::Log::default(),
                        ));
                }
            )
            .add_observer(|
                    client_remove: prelude::On<prelude::Remove, spru_bevy::client::component::Runner<crate::Client>>,
                    mut commands: prelude::Commands,
                    mut client_map: prelude::ResMut<ClientMap>,
                | {
                    client_map.remove(client_remove.entity);

                    // TODO don't destroy the Log for now, because spru_bevy currently removes and reinserts the Runner 
                    // every frame in order to split the World into Runner and The Rest of the World.
                    // commands.entity(client_remove.entity)
                    //     .remove::<(
                    //         crate::Log,
                    //     )>();
                }
            )
            .add_observer(
                |stage_interaction: prelude::On<spru_bevy::client::event::StageInteraction<crate::Client>>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let message = match &stage_interaction.result {
                        Ok(pending_id) => format!("Interaction staged ({pending_id})"),
                        Err(err) => format!("Stage failed: {err}"),
                    };
                    let mut log = q_log.get_mut(stage_interaction.entity).ok();
                    crate::Log::try_log(&mut log, message);
                },
            )
            .add_observer(
                |apply_interactions: prelude::On<spru_bevy::client::event::ApplyInteractions<crate::Client>>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let message = match &apply_interactions.result {
                        Ok(count) => {
                            if *count == 0 {
                                return;
                            }
                            format!("{count} Interactions applied")
                        }
                        Err(err) => format!("Apply failed: {err}"),
                    };
                    let mut log = q_log.get_mut(apply_interactions.entity).ok();
                    crate::Log::try_log(&mut log, message);
                },
            )
            .add_observer(
                |revert_interactions: prelude::On<spru_bevy::client::event::RevertInteractions<crate::Client>>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let message = match &revert_interactions.result {
                        Ok(count) => {
                            if *count == 0 {
                                return;
                            }
                            format!("{count} Interactions reverted")
                        }
                        Err(err) => format!("Revert failed: {err}"),
                    };
                    let mut log = q_log.get_mut(revert_interactions.entity).ok();
                    crate::Log::try_log(&mut log, message);
                },
            )
        ;
    }
}

#[derive(Debug, Default)]
#[derive(prelude::Resource)]
pub(crate) struct ClientMap {
    map: HashMap<(spru::game::Id, spru::player::Id), prelude::Entity>,
    inverse_map: HashMap<prelude::Entity, (spru::game::Id, spru::player::Id)>,
}

impl ClientMap {
    fn insert(&mut self, game_id: spru::game::Id, player_id: spru::player::Id, entity: prelude::Entity) {
        self.map.insert((game_id, player_id), entity);
        self.inverse_map.insert(entity, (game_id, player_id));
    }

    fn remove(&mut self, entity: prelude::Entity) {
        if let Some(ids) = self.inverse_map.remove(&entity) {
            self.map.remove(&ids);
        }
    }

    pub(crate) fn get(&self, game_id: spru::game::Id, player_id: spru::player::Id) -> Option<prelude::Entity> {
        self.map.get(&(game_id, player_id)).copied()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (spru::game::Id, spru::player::Id, prelude::Entity)> {
        self.map.iter()
            .map(|(&(game_id, player_id), &entity)| (game_id, player_id, entity))
    }
}
