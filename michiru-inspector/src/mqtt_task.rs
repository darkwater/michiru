use std::time::Duration;

use chrono::Local;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use tokio::sync::mpsc::UnboundedSender;

use crate::topic_tree::TopicValue;

pub fn run(tx: UnboundedSender<TopicValue>) {
    tokio::task::spawn(async move {
        let mut mqttoptions = MqttOptions::new("michiru", "michiru.fbk.red", 1883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        mqttoptions.set_max_packet_size(1024 * 1024, 1024 * 1024);

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
        client.subscribe("#", QoS::ExactlyOnce).await.unwrap();

        loop {
            let notification = eventloop.poll().await.unwrap();
            tracing::trace!(?notification);

            if let Event::Incoming(Packet::Publish(obj)) = notification {
                tx.send(TopicValue {
                    topic: obj.topic,
                    payload: obj.payload.into(),
                    retain: obj.retain,
                    received: Local::now(),
                })
                .unwrap();
            }
        }
    });
}
