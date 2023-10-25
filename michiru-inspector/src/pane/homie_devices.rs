use crate::state::AppState;

pub struct HomieDevices;

impl HomieDevices {
    pub fn ui(&mut self, ui: &mut egui::Ui, state: &mut AppState) {
        ui.heading("Homie Devices");
    }
}
