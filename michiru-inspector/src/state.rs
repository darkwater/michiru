use crate::topic_tree::{TopicTree, TopicValue};

#[derive(Default)]
pub struct AppState {
    pub topic_tree: TopicTree,
    pub selected: Option<TopicValue>,
}
