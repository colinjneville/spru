fn remote_join_ui(&mut self, ui: &mut egui::Ui) {
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