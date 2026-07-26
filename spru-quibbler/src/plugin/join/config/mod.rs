mod remote_join;

use bevy::prelude;

#[derive(Debug, Default)]
pub(super) struct Config {
    step: usize,
    player_panel: super::ConfigPlayerPanel,
    server_panel: ConfigRemoteJoinPanel,
}

impl super::Config for Config {
    fn title(&self) -> &'static str {
        "Join Remote Game"
    }

    fn panels(&mut self) -> (Vec<&mut dyn super::ConfigPanel>, &mut usize) {
        (
            vec![
                &mut self.player_panel as &mut dyn super::ConfigPanel,
                &mut self.server_panel,
            ],
            &mut self.step,
        )
    }

    fn complete(self: Box<Self>, commands: &mut prelude::Commands) {
        let username = self.player_panel.username.into_value();
        let mut address = self.server_panel.address.into_value();
        let cert_hash = self.server_panel.cert_hash.into_value();
        let port = self.server_panel.port.into_value();
        let password = self.server_panel.password;

        if address.set_port(Some(port)).is_err() {
            prelude::warn!("Can't set port for address {address}");
        }

        let cert_hash = Some(cert_hash.0).filter(|ch| ch != &CertificateHash::default());

        let mut connection_config = spru_bevy::client::remote::component::ConnectionConfig::new(address);
        connection_config.certificate_hash = cert_hash;
        connection_config.headers.insert("username".to_string(), username);
        if !password.is_empty() {
            connection_config.headers.insert("password".to_string(), password);
        }
            
        let join_remote = spru_bevy::client::remote::command::JoinRemote::<crate::Client>::new(connection_config);
            
        commands
            .spawn_empty()
            .queue(plugin::join::StartJoinLobby { })
            .queue(join_remote);
    }
}