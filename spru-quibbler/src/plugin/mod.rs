mod core;
pub(crate) use core::Core;
cfg_select! {
    feature = "client" => {
        mod client;
        pub(crate) use client::Client;
    }
    _ => {
        pub(crate) use Noop as Client;
    }
}
cfg_select! {
    feature = "dedicated-client" => {
        mod dedicated_client;
        pub(crate) use dedicated_client::DedicatedClient;
    }
    _ => {
        pub(crate) use Noop as DedicatedClient;
    }
}
cfg_select! {
    feature = "dedicated-server" => {
        mod dedicated_server;
        pub(crate) use dedicated_server::DedicatedServer;
    }
    _ => {
        pub(crate) use Noop as DedicatedServer;
    }
}
cfg_select! {
    feature = "local" => {
        mod local;
        pub(crate) use local::Local;
    }
    _ => {
        pub(crate) use Noop as Local;
    }
}
cfg_select! {
    feature = "remote" => {
        mod remote;
        pub(crate) use remote::Remote;
    }
    _ => {
        pub(crate) use Noop as Remote;
    }
}
cfg_select! {
    feature = "server" => {
        mod server;
        pub(crate) use server::Server;
    }
    _ => {
        pub(crate) use Noop as Server;
    }
}

cfg_select! {
    feature = "ui" => {
        mod ui;
        pub(crate) use ui::Ui;
    }
    _ => {
        pub(crate) use Noop as Ui;
    }
}

use bevy::prelude;

#[allow(dead_code, reason = "Replaces plugins for disabled features")]
pub(crate) struct Noop;

impl prelude::Plugin for Noop {
    fn build(&self, _app: &mut bevy::app::App) { }
    
    fn is_unique(&self) -> bool {
        false
    }
}
