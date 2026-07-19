use bevy::prelude;
use bevy_egui::egui;

pub(super) struct Data {
    client_entity: prelude::Entity,
    usernames: Vec<String>,
    local_index: Option<usize>,
    has_started: bool,
}

pub(super) fn join_lobby_connecting_ui(
    mut egui: bevy_egui::EguiContexts,
    timer: prelude::Res<prelude::Time>,
) -> prelude::Result {
    let ctx = egui.ctx_mut()?;

    let dots = (timer
        .elapsed()
        .subsec_micros()
        % 4) as usize;

    egui::Window::new("join_lobby_connecting")
        .title_bar(false)
        .movable(false)
        .min_width(480.)
        .show(ctx, |ui| {
            let mut s = "Connecting".to_string();
            for _ in 0..dots {
                s.push('.');
            }
            ui.label(s);
        })
        ;

    Ok(())
}

pub(super) fn join_lobby_ui_get_client(
    active_game: prelude::Res<super::ActiveGame>,
    active_client: prelude::Res<super::ActiveClient>,
    client_map: prelude::Res<spru_bevy::client::resource::ClientMap>,
    q_has_join_lobby: prelude::Query<(

    ), (
        prelude::With<crate::plugin::remote_client::JoinLobby>,
    )>,
) -> Option<(spru::player::Id, prelude::Entity)> {
    let game_id = active_game.0?;
    let client_id = active_client.get(game_id)?;
    let client_entity = client_map.get(game_id, client_id)?;
    let () = q_has_join_lobby.get(client_entity).ok()?;

    Some((client_id, client_entity))
}

pub(super) fn join_lobby_ui_get_data(
    prelude::In(input): prelude::In<Option<(spru::player::Id, prelude::Entity)>>,
    world: &prelude::World,
) 
    -> Option<Data> 
{
    use spru_bevy::client::IdTExt as _;

    let (client_id, client_entity) = input?;

    let root = world.entity(client_entity)
        .get_components::<&spru_bevy::common::component::Root<crate::Common>>()
        .ok()?
        .0.from_world(world, client_entity);

    let has_started = root.has_started;

    let players = root
        .players
        .from_world(world, client_entity);

    let mut usernames = vec![];
    let mut local_index = None;
    for (i, (player_id, player_root)) in players.iter().enumerate() {
        usernames.push(player_root.data.username.clone());
        if player_id == client_id {
            local_index = Some(i);
        }
    }

    Some(Data {
        client_entity,
        usernames,
        local_index,
        has_started,
    })
}


pub(super) fn join_lobby_ui(
    prelude::In(data): prelude::In<Option<Data>>,
    mut commands: prelude::Commands,
    mut egui: bevy_egui::EguiContexts,
    mut next_state: prelude::ResMut<prelude::NextState<crate::AppState>>,
) {
    if let Some(data) = data {
        let Data {
            client_entity,
            usernames,
            local_index,
            has_started,
        } = data;

        if has_started {
            next_state.set_if_neq(crate::AppState::InGame);
        }

        if let Ok(ctx) = egui.ctx_mut() {
            // This is a lazy way of recreating a lobby without any additional non-game messages from the server
            egui::Window::new("join_lobby")
                .title_bar(false)
                .movable(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Waiting for host to begin game...");
                    });
                    for (index, username) in usernames.into_iter().enumerate() {
                        ui.horizontal(|ui| {
                            if Some(index) == local_index {
                                ui.colored_label(egui::Color32::GOLD, username);
                            } else {
                                ui.label(username);
                            }
                        });
                    }
                    if ui.button("Cancel")
                        .clicked()
                    {
                        commands
                            .entity(client_entity)
                            .queue(
                                spru_bevy::client::remote::command::Disconnect::<crate::Client>::new("User cancelled".to_string())
                            );
                    }
                });
        }
    }
}

// fn in_game_reconnect(
//     mut egui: bevy_egui::EguiContexts,
//     q_connecting: prelude::Query<(
//         &spru_bevy::remote::wtransport::quinn::Connecting,
//         &spru_bevy::common::component::GameId,
//         &spru_bevy::client::component::ClientId,
//     )>,
// ) -> prelude::Result {
//     for (_connecting, game_id, client_id) in q_connecting {
//         egui::Window::new("in_game_reconnect")
//             .title_bar(false)
//             .resizable(false)
//             .show(|ui| {
//                 "Lost connection"
//             })
//             ;
//     }

//     Ok(())
// }
