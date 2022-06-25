use super::routes::{DoorSensorHandler, MotionSensorHandler, SwitchHandler};
use crate::configuration::AppConfig;
use log::*;
use mqtt_router::Router;
use rumqttc::{AsyncClient, ConnAck, Event, Incoming, MqttOptions, Publish, QoS, SubscribeFilter};
use serenity::http::Http;
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::unbounded_channel;

enum MqttUpdate {
    Message(Publish),
    Reconnection(ConnAck),
}

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
            match eventloop.poll().await {
                Ok(notification) => match notification {
                    Event::Incoming(Incoming::Publish(publish)) => {
                        if let Err(e) = message_sender.send(MqttUpdate::Message(publish)) {
                            eprintln!("Error sending message {}", e);
                        }
                    }
                    Event::Incoming(Incoming::ConnAck(con_ack)) => {
                        if let Err(e) = message_sender.send(MqttUpdate::Reconnection(con_ack)) {
                            eprintln!("Error sending message {}", e);
                        }
                    }
                    _ => (),
                },
                Err(e) => {
                    eprintln!("Error processing eventloop notifications {}", e);
                }
            }
        }
    });

    tokio::spawn(async move {
        let mut router = Router::default();

        router
            .add_handler(
                "zigbee2mqtt/main_door",
                DoorSensorHandler::new(
                    discord_http.clone(),
                    app_config.home.notification_discord_channel,
                ),
            )
            .unwrap();

        router
            .add_handler(
                "zigbee2mqtt/switch/#",
                SwitchHandler::new(
                    discord_http.clone(),
                    app_config.home.notification_discord_channel,
                ),
            )
            .unwrap();

        let topics = router
            .topics_for_subscription()
            .map(|topic| SubscribeFilter {
                path: topic.to_owned(),
                qos: QoS::AtMostOnce,
            });
        client.subscribe_many(topics).await.unwrap();

        loop {
            let update = message_receiver.recv().await.unwrap();
            match update {
                MqttUpdate::Message(message) => {
                    match router
                        .handle_message_ignore_errors(&message.topic, &message.payload)
                        .await
                    {
                        Ok(false) => error!("No handler for topic: \"{}\"", &message.topic),
                        Ok(true) => (),
                        Err(e) => error!("Failed running handler with {:?}", e),
                    }
                }
                MqttUpdate::Reconnection(_) => {
                    info!("Reconnecting to broker");
                    let topics = router
                        .topics_for_subscription()
                        .map(|topic| SubscribeFilter {
                            path: topic.to_owned(),
                            qos: QoS::AtMostOnce,
                        });
                    client.subscribe_many(topics).await.unwrap();
                }
            }
        }
    });

    Ok(())
}
