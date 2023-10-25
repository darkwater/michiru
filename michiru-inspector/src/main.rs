mod mqtt_task;
mod pane;
mod state;
mod topic_tree;

use std::time::Duration;

use eframe::CreationContext;
use egui_dock::{DockArea, DockState};
use tokio::sync::mpsc::UnboundedReceiver;

use self::{pane::Pane, state::AppState, topic_tree::TopicValue};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    mqtt_task::run(tx);

    tokio::task::block_in_place(|| {
        let native_options = eframe::NativeOptions::default();
        eframe::run_native(
            "Michiru Inspector",
            native_options,
            Box::new(|cc| Box::new(InspectorApp::new(cc, rx))),
        )
        .unwrap();
    });
}

struct InspectorApp {
    state: AppState,
    dock_state: DockState<Pane>,
}

impl InspectorApp {
    fn new(cc: &CreationContext, rx: UnboundedReceiver<TopicValue>) -> Self {
        let dock_state = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, "dock_state"))
            .unwrap_or_else(|| DockState::new(Pane::all()));

        Self { state: AppState::new(rx), dock_state }
    }
}

impl eframe::App for InspectorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.state.update();

        DockArea::new(&mut self.dock_state).show(ctx, &mut TabViewer { state: &mut self.state });

        ctx.request_repaint_after(Duration::from_millis(50));
    }
}

struct TabViewer<'a> {
    state: &'a mut AppState,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Pane;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            Pane::MqttTopics => "MQTT Topics".into(),
            Pane::HomieDevices => "Homie Devices".into(),
            Pane::Zigbee2Mqtt(_) => "Zigbee2Mqtt".into(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        tab.ui(ui, self.state);
    }
}
