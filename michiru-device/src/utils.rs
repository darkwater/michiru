pub fn valid_topic_id(topic: &str) -> bool {
    !topic.is_empty()
        && !topic.starts_with('-')
        && !topic.ends_with('-')
        && topic
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}
