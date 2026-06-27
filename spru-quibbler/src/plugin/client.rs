use bevy::prelude;

pub(crate) struct Client;

impl prelude::Plugin for Client {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .add_plugins((
                spru_bevy::client::Plugin::<crate::Client>::default(),
            ))
            .add_observer(|
                    client_add: prelude::On<prelude::Add, spru_bevy::client::component::Runner<crate::Client>>,
                    mut commands: prelude::Commands,
                | {
                    commands.entity(client_add.entity)
                        .insert_if_new((
                            crate::Log::default(),
                        ));
                }
            )
            .add_observer(|
                    client_remove: prelude::On<prelude::Remove, spru_bevy::client::component::Runner<crate::Client>>,
                    mut commands: prelude::Commands,
                | {
                    commands.entity(client_remove.entity)
                        .remove::<(
                            crate::Log,
                        )>();
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
