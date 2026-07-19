use bevy::prelude;
use bevy_egui::egui;

pub fn host_lobby_ui(
    mut commands: prelude::Commands,
    mut egui: bevy_egui::EguiContexts,
    server_map: prelude::Res<spru_bevy::server::resource::ServerMap>,
    active_game: prelude::Res<super::ActiveGame>,
    q_host_lobby: prelude::Query<(
        &crate::plugin::remote_server::HostLobby,
        &spru_bevy::remote::component::CertificateHash,
        &spru_bevy::server::remote::component::Listener,
    )>,
    q_listener: prelude::Query<(
        &spru_bevy::remote::aeronet::io::connection::LocalAddr,
    )>,
    mut clipboard: prelude::ResMut<prelude::Clipboard>,
    mut external_ip: prelude::ResMut<crate::plugin::remote_server::ExternalIp>,
) {
    let Some(active_game) = active_game.0 else {
        return
    };
    let Some(server_entity) = server_map.get(active_game) else {
        return
    };
    let Ok((host_lobby, certificate_hash, listener_entity, )) = q_host_lobby.get(server_entity) else {
        return
    };
    let Ok((local_addr, )) = q_listener.get(listener_entity.listener()) else {
        return
    };
    let Ok(ctx) = egui.ctx_mut() else {
        return
    };

    let hash = certificate_hash.to_base64();

    egui::Panel::left("host_lobby")
        .show(ctx, |ui| {
            if let Some(ip) = external_ip.get() {
                ui.horizontal(|ui| {
                    if ui.label(ip)
                        .on_hover_text("Click to copy to clipboard")
                        .clicked()
                    {
                        if let Err(err) = clipboard.set_text(ip) {
                            prelude::warn!("Failed to copy IP to clipboard: {err}");
                        }
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.label(format!("port {}", local_addr.port()));
            });
            ui.horizontal(|ui| {
                if ui.label(format!("Certificate Hash: {hash}"))
                    .on_hover_text("Click to copy to clipboard")
                    .clicked()
                {
                    if let Err(err) = clipboard.set_text(hash) {
                        prelude::warn!("Failed to copy certificate hash to clipboard: {err}");
                    }
                }
            });

            for (i, request) in host_lobby.players.iter().enumerate() {
                ui.horizontal(|ui| {
                    // Don't allow kicking yourself (i == 0)
                    if i > 0 {
                        if ui.button("X").clicked() {
                            commands
                                .entity(server_entity)
                                .queue(spru_bevy::server::command::RemovePlayer::<crate::Server>::new(request.player_id));
                        }

                        ui.label(request.username.clone());
                    } else {
                        ui.colored_label(egui::Color32::GOLD, request.username.clone());
                    }
                });
            }
            for _i in host_lobby.players.len()..host_lobby.max_players {
                ui.horizontal(|ui| {
                    let label = egui::Label::new("Empty");
                    ui.add_enabled(false, label);
                });
            }
            ui.horizontal(|ui| {
                if ui.button("Start Game").clicked() {
                    commands
                        .entity(server_entity)
                        .queue(crate::plugin::remote_server::StartGame {
                            
                        })
                    ;
                }
            })
        });
}