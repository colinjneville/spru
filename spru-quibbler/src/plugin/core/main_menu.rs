use bevy::{ecs::{schedule::IntoSystemSet as _, system::EntityCommand as _}, prelude};

use crate::plugin::{self, core::button};

#[derive(Debug)]
#[derive(prelude::Component, prelude::Reflect)]
pub struct MainMenu;

pub(super) fn make_main_menu(
    mut commands: prelude::Commands,
) {
    let mut main_menu = commands
        .spawn((
            MainMenu,
        ));

    // main_menu.world_scope(|world| {

    // });
    let exit_id = main_menu
        .commands()
        .register_system(exit);
    main_menu.queue(button::Add::new("Exit", exit_id));
    

    // #[cfg(feature = "hotseat")]
    // commands
    //     .spawn_empty()
    //     .queue(super::button::Add::new(main_menu))
    //     .observe(plugin::local::)

    #[cfg(feature = "hotseat")]
    {
        if ui.button("Local Game").clicked() {
            config_state.set(hotseat::ConfigLocal::default());
            next_state.set_if_neq(crate::AppState::Config);
        }
        ui.end_row();
    }
    #[cfg(feature = "host")]
    {
        if ui.button("Host Game").clicked() {
            config_state.set(host::ConfigHost::new(external_ip.get()));
            next_state.set_if_neq(crate::AppState::Config);
        }
        ui.end_row();
    }
    #[cfg(feature = "join")]
    {
        if ui.button("Join Game").clicked() {
            config_state.set(join::ConfigJoin::default());
            next_state.set_if_neq(crate::AppState::Config);
        }
        ui.end_row();
    }
    #[cfg(all(feature = "dedicated-server"))]
    {
        if ui.button("Dedicated Server").clicked() {
            config_state.set(dedicated_server::ConfigDedicatedServer::default());
            next_state.set_if_neq(crate::AppState::Config);
        }
        ui.end_row();
    }
    {
        if ui.button("Exit").clicked() {
            app_exit.write_default();
        }
        ui.end_row();
    }
}

fn exit(
    mut app_exit: prelude::MessageWriter<prelude::AppExit>,
) {
    app_exit.write_default();
}