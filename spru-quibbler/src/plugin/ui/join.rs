use std::fmt;

use bevy::prelude;
use bevy_egui::egui;

use spru_bevy::remote::component::CertificateHash;

use super::ConfigPanel as _;

use super::Validated;

#[derive(Debug, Default)]
struct BlankableCertificateHash(CertificateHash);

impl fmt::Display for BlankableCertificateHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == CertificateHash::default() {
            Ok(())
        } else {
            write!(f, "{}", self.0)
        }
    }
}

#[derive(Debug)]
struct ConfigRemoteJoinPanel {
    address: Validated<url::Url>,
    port: Validated<u16>,
    cert_hash: Validated<BlankableCertificateHash>,
    password: String,
}

impl Default for ConfigRemoteJoinPanel {
    fn default() -> Self {
        Self { 
            address: Validated::new(url::Url::parse("https://localhost").unwrap()), 
            port: Validated::new(super::DEFAULT_PORT), 
            cert_hash: Validated::new(BlankableCertificateHash::default()),
            password: String::new(),
        }
    }
}

impl ConfigRemoteJoinPanel {
    fn validate_address(s: &str) -> Result<url::Url, &'static str> {
        let mut url = url::Url::parse(s)
            .map_err(|_| "Invalid address")?;

        // Test if we can set a port
        url.set_port(url.port())
            .map_err(|_| "Invalid address")?;

        Ok(url)
    }

    fn validate_hash(s: &str) -> Result<BlankableCertificateHash, &'static str> {
        if s.is_empty() {
            Ok(BlankableCertificateHash::default())
        } else {
            spru_bevy::remote::aeronet_webtransport::cert::hash_from_b64(s)
                .map(CertificateHash)
                .map(BlankableCertificateHash)
                .map_err(|_| "Invalid certificate hash")
        }
    }
}

impl super::ConfigPanel for ConfigRemoteJoinPanel {
    fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Server Address");
            if ui.text_edit_singleline(self.address.text_mut()).lost_focus() {
                self.address.validate(Self::validate_address);
            }
        });
        if let Some(address_error) = self.address.error() {
            ui.colored_label(egui::Color32::RED, address_error);
            ui.end_row();
        }

        ui.horizontal(|ui| {
            ui.label("Port");
            if ui.text_edit_singleline(self.port.text_mut()).lost_focus() {
                self.port.validate(super::validate_port);
            }
        });
        if let Some(port_error) = self.port.error() {
            ui.colored_label(egui::Color32::RED, port_error);
            ui.end_row();
        }

        ui.horizontal(|ui| {
            ui.label("Certificate Hash");
            if ui.text_edit_singleline(self.cert_hash.text_mut()).lost_focus() {
                self.cert_hash.validate(Self::validate_hash);
            }
        });
        if let Some(cert_hash_error) = self.cert_hash.error() {
            ui.colored_label(egui::Color32::RED, cert_hash_error);
            ui.end_row();
        }

        ui.horizontal(|ui| {
            ui.label("Password");
            egui::TextEdit::singleline(&mut self.password)
                .hint_text("Optional")
                .show(ui);
        });
    }

    fn valid(&self) -> Result<(), std::borrow::Cow<'static, str>> {
        Ok(())
    }
}

#[derive(Debug, Default)]
pub(super) struct ConfigJoin {
    step: usize,
    player_panel: super::ConfigPlayerPanel,
    server_panel: ConfigRemoteJoinPanel,
}

impl super::Config for ConfigJoin {
    fn show(&mut self, ctx: &mut egui::Context) -> bool {
        self.player_panel.show_panel(&mut self.step, 0, ctx, "Next");
        self.server_panel.show_panel(&mut self.step, 1, ctx, "Join");
        self.step == 2
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
            .queue(crate::plugin::remote_client::StartJoinLobby { })
            .queue(join_remote);
    }
}