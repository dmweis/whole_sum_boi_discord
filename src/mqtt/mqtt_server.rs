use super::{router::Router, routes::SimpleHandler};
use crate::configuration::AppConfig;
use log::*;
use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};
use serenity::http::Http;
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::unbounded_channel;
use tokio::sync::Mutex;

pub fn start_mqtt_service(app_config: AppConfig, discord_http: Arc<Http>) -> anyhow::Result<()> {
    let mut mqttoptions = MqttOptions::new(
        &app_config.mqtt.client_id,
        &app_config.mqtt.broker_host,
        app_config.mqtt.broker_port,
    );
    info!("Starting MQTT server with options {:?}", mqttoptions);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    let base_topic = app_config.mqtt.base_route;

    info!("MQTT base topic {}", base_topic);

    let (message_sender, mut message_receiver) = unbounded_channel();

    tokio::spawn(async move {
        loop {
            if let Ok(notification) = eventloop.poll().await {
                if let Event::Incoming(Incoming::Publish(publish)) = notification {
                    message_sender
                        .send(publish)
                        .expect("Failed to publish message");
                }
            } else {
                error!("failed processing mqtt notifications");
            }
        }
    });

    tokio::spawn(async move {
        let mut router = Router::default();

        let simple_route = format!("{}/simple", base_topic);
        client
            .subscribe(&simple_route, QoS::AtMostOnce)
            .await
            .unwrap();
        router.add_handler(&simple_route, SimpleHandler::new());

        loop {
            let message = message_receiver.recv().await.unwrap();
            match router
                .handle_message(message.topic.clone(), &message.payload)
                .await
            {
                Ok(false) => error!("No handler for topic: \"{}\"", &message.topic),
                Ok(true) => (),
                Err(e) => error!("Failed running handler with {:?}", e),
            }
        }
    });

    Ok(())
}
