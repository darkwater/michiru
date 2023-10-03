use std::collections::BTreeMap;

use bytes::Bytes;
use chrono::{DateTime, Local};
use egui::{collapsing_header::CollapsingState, Id, Ui};
use serde_json::Value;

const SEPARATOR: char = '/';

// #[derive(Default)]
// pub struct TopicTree {
//     pub root: TopicNode,
// }

#[derive(Default)]
pub struct TopicTree {
    pub value: Option<TopicValue>,
    pub children: BTreeMap<String, TopicTree>,
}

impl TopicTree {
    pub fn insert(&mut self, value: TopicValue) {
        // if value.topic != topic {
        //     tracing::warn!(
        //         "Topic mismatch: value.topic = {:?}, topic = {:?}",
        //         value.topic,
        //         topic
        //     );
        // }

        let mut node = self;
        for part in value.topic.split(SEPARATOR) {
            node = node.children.entry(part.to_string()).or_default();
        }
        node.value = Some(value);
    }

    pub fn get(&self, topic: &str) -> Option<&TopicValue> {
        let mut node = self;
        for part in topic.split(SEPARATOR) {
            node = node.children.get(part)?;
        }
        node.value.as_ref()
    }

    pub fn show(&self, ui: &mut Ui, selected_topic: &mut Option<TopicValue>) {
        for (topic, tree) in &self.children {
            let mut heading = |ui: &mut Ui| {
                let heading = topic.to_string();

                if let Some(value) = tree.value.as_ref() {
                    let res = ui.selectable_label(
                        selected_topic.as_ref().map(|t| &t.topic) == Some(&value.topic),
                        heading,
                    );

                    ui.label("=");
                    ui.label(match &value.payload {
                        TopicPayload::String(value) => value.into(),
                        TopicPayload::Json(Value::String(value)) => value.clone(),
                        TopicPayload::Json(Value::Number(value)) => value.to_string(),
                        TopicPayload::Json(Value::Bool(value)) => value.to_string(),
                        TopicPayload::Json(Value::Null) => "null".into(),
                        _ => "...".into(),
                    });

                    if res.clicked() {
                        *selected_topic = tree.value.clone();
                    }
                } else {
                    ui.set_enabled(false);
                    let _ = ui.selectable_label(false, heading);
                }
            };

            if tree.children.is_empty() {
                ui.horizontal(|ui| {
                    ui.add_space(20.);
                    heading(ui);
                });
            } else {
                CollapsingState::load_with_default_open(
                    ui.ctx(),
                    Id::new(ui.id().with(topic)),
                    false,
                )
                .show_header(ui, |ui| {
                    heading(ui);
                })
                .body(|ui| {
                    tree.show(ui, selected_topic);
                });
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct TopicValue {
    pub topic: String,
    pub payload: TopicPayload,
    pub retain: bool,
    pub received: DateTime<Local>,
}

#[derive(Clone, Debug)]
pub enum TopicPayload {
    Json(serde_json::Value),
    String(String),
    Bytes(Vec<u8>),
}

impl From<Bytes> for TopicPayload {
    fn from(value: Bytes) -> Self {
        if let Ok(value) = serde_json::from_slice(&value) {
            Self::Json(value)
        } else if let Ok(value) = String::from_utf8(value.to_vec()) {
            Self::String(value)
        } else {
            Self::Bytes(value.to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;

    fn value(topic: &'static str) -> TopicValue {
        TopicValue {
            topic: topic.into(),
            payload: TopicPayload::String("".into()),
            retain: false,
            received: Local::now(),
        }
    }

    #[test]
    fn test_name() {
        let mut tree = TopicTree::default();
        assert!(tree.children.is_empty());
        assert!(tree.value.is_none());

        tree.insert(value("foo"));
        tree.insert(value("bar"));
        tree.insert(value("foo/bar"));
        assert_eq!(tree.children.len(), 2);
        assert_eq!(tree.children["foo"].children.len(), 1);
        assert_eq!(tree.get("foo").unwrap().topic, "foo");
        assert_eq!(tree.get("bar").unwrap().topic, "bar");
        assert_eq!(tree.get("foo/bar").unwrap().topic, "foo/bar");

        assert_eq!(tree.children.keys().collect_vec(), vec!["bar", "foo"]);
    }
}
