mod mqtt_task;
mod state;
mod topic_tree;

use std::time::Duration;

use chrono::Local;
use egui::ScrollArea;
use egui_json_tree::{DefaultExpand, JsonTree};
use tokio::sync::mpsc::UnboundedReceiver;

use self::{
    state::AppState,
    topic_tree::{TopicPayload, TopicValue},
};

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
    rx: UnboundedReceiver<TopicValue>,
    state: AppState,
}

impl InspectorApp {
    fn new(cc: &eframe::CreationContext<'_>, rx: UnboundedReceiver<TopicValue>) -> Self {
        Self {
            rx,
            state: AppState::default(),
        }
    }
}

impl eframe::App for InspectorApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        while let Ok(value) = self.rx.try_recv() {
            self.state.topic_tree.insert(value);
        }

        egui::SidePanel::left("tree").show(ctx, |ui| {
            ScrollArea::new([true; 2])
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    self.state.topic_tree.show(ui, &mut self.state.selected);
                });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::new([true; 2])
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    if let Some(value) = &self
                        .state
                        .selected
                        .as_ref()
                        .and_then(|v| self.state.topic_tree.get(&v.topic))
                    {
                        let seconds = Local::now()
                            .signed_duration_since(value.received)
                            .num_seconds();

                        ui.heading(&value.topic);
                        ui.add_space(5.);
                        ui.label(format!("{seconds} seconds ago"));
                        ui.add_space(5.);
                        match &value.payload {
                            TopicPayload::Json(v) => {
                                JsonTree::new(&value.topic, v)
                                    .default_expand(DefaultExpand::ToLevel(0))
                                    .show(ui);
                            }
                            TopicPayload::String(v) => {
                                ui.label(v);
                            }
                            TopicPayload::Bytes(v) => {
                                ui.label(format!("{:02x?}", v));
                            }
                        };
                    }
                });
        });

        ctx.request_repaint_after(Duration::from_millis(50));
    }
}
