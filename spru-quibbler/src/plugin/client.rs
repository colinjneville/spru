use bevy::prelude;

pub(crate) struct Client;

impl prelude::Plugin for Client {
    fn build(&self, app: &mut prelude::App) {
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
                    commands
                        .entity(client_remove.entity)
                        .try_remove::<(
                            crate::Log,
                        )>();
                }
            )
            .add_observer(
                |interaction_staged: prelude::On<spru_bevy::client::event::InteractionStaged<crate::Client>>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let message = format!("Interaction staged ({})", interaction_staged.pending_id);

                    let mut log = q_log.get_mut(interaction_staged.entity).ok();
                    crate::Log::try_log(&mut log, message);
                },
            )
            .add_observer(
                |interaction_stage_error: prelude::On<spru_bevy::client::event::InteractionStageError<crate::Client>>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let message = format!("Stage failed: {}", interaction_stage_error.error);
                    
                    let mut log = q_log.get_mut(interaction_stage_error.entity).ok();
                    crate::Log::try_log(&mut log, message);
                },
            )
            .add_observer(
                |interactions_applied: prelude::On<spru_bevy::client::event::InteractionsApplied>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    if interactions_applied.count > 0 {
                        let message = format!("{} Interactions applied", interactions_applied.count);
                        
                        let mut log = q_log.get_mut(interactions_applied.entity).ok();
                        crate::Log::try_log(&mut log, message);
                    }
                },
            )
            .add_observer(
                |interactions_apply_error: prelude::On<spru_bevy::client::event::InteractionsApplyError>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let message = format!("Apply failed: {}", interactions_apply_error.error);
                    
                    let mut log = q_log.get_mut(interactions_apply_error.entity).ok();
                    crate::Log::try_log(&mut log, message);
                },
            )
            .add_observer(
                |interactions_reverted: prelude::On<spru_bevy::client::event::InteractionsReverted>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    if interactions_reverted.count > 0 {
                        let message = format!("{} Interactions reverted", interactions_reverted.count);
                        
                        let mut log = q_log.get_mut(interactions_reverted.entity).ok();
                        crate::Log::try_log(&mut log, message);
                    }
                },
            )
            .add_observer(
                |interactions_revert_error: prelude::On<spru_bevy::client::event::InteractionsRevertError>,
                mut q_log: prelude::Query<&mut crate::Log>|
                {
                    let message = format!("Revert failed: {}", interactions_revert_error.error);

                    let mut log = q_log.get_mut(interactions_revert_error.entity).ok();
                    crate::Log::try_log(&mut log, message);
                },
            )
        ;
    }
}
