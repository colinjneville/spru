use bevy::prelude;

pub(crate) struct Local;

impl prelude::Plugin for Local {
    fn build(&self, app: &mut prelude::App) {
        app
            .add_plugins((
                spru_bevy::local::Plugin::<crate::Server, crate::Client>::default(),
            ))
        ;
    }
}