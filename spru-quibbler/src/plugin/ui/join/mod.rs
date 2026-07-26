mod config;
mod connecting;
mod join_lobby;

use std::fmt;

use bevy::{ecs::system::IntoSystem as _, prelude};
use bevy_egui::egui;

use spru_bevy::remote::component::CertificateHash;

use crate::plugin;

use super::Validated;

#[derive(Debug)]
pub(super) struct Plugin;

impl prelude::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app
            .add_systems(bevy_egui::EguiPrimaryContextPass, (
                connecting::ui.pipe(crate::error_to_console),
                join_lobby::ui_get_client
                    .pipe(join_lobby::ui_get_data)
                    .pipe(join_lobby::ui_render),
            ))
            .add_observer(join_lobby::on_insert)
            ;
    }
}


