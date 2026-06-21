use bevy::prelude;

pub(crate) struct Core;

impl Core {
    fn print_piles(
        q_piles: prelude::Query<
            (
                &spru_bevy::client::component::ClientId,
                &spru_bevy::client::component::Item<spru_util::pile::Pile<crate::data::Card>>,
            ),
            (prelude::Changed<spru_bevy::client::component::Item<spru_util::pile::Pile<crate::data::Card>>>,),
        >,
    ) {
        for (client_id, pile) in q_piles {
            let mut s = String::new();
            for card in &**pile {
                s.push_str(&format!("{} ", card.face().letters_str()));
            }
            prelude::trace!(name: "pile_changed", client = client_id.into_u32(), value = s);
        }
    }
}

impl prelude::Plugin for Core {
    fn build(&self, app: &mut bevy::app::App) {
        use prelude::PluginGroup as _;

        let _frame_duration = std::time::Duration::from_secs_f32(1. / 30.);

        app
            .add_plugins((
                bevy::DefaultPlugins.set(bevy::log::LogPlugin {
                    filter: "spru=info,spru_bevy=info,spru_quibbler=trace".to_string(),
                    ..Default::default()
                }),
            ))
            .add_systems(prelude::FixedUpdate, (
                Self::print_piles,
            ))
        ;
    }
}
