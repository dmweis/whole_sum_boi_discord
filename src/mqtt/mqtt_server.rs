use super::routes::{
    DiscordChannelMessageHandler, DoorSensorHandler, MotionSensorHandler, SwitchHandler,
};
use crate::{
    configuration::AppConfig,
    mqtt::routes::{DiscordChannelFileMessageHandler, DiscordChannelShowTypingHandler},
};
use log::*;
use mqtt_router::Router;
use rumqttc::{AsyncClient, ConnAck, Event, Incoming, MqttOptions, Publish, QoS, SubscribeFilter};
use serde::Serialize;
use serenity::model::channel::Message;
use serenity::{http::Http, model::prelude::Attachment};
use std::{sync::Arc, time::Duration};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

const MQTT_MAX_PACKET_SIZE: usize = 268435455;

enum MqttUpdate {
    Message(Publish),
    Reconnection(ConnAck),
}

pub fn start_mqtt_service(
    app_config: AppConfig,
    discord_http: Arc<Http>,
    mut discord_message_receiver: UnboundedReceiver<Message>,
) -> anyhow::Result<()> {
    let mut mqttoptions = MqttOptions::new(
        &app_config.mqtt.client_id,
        &app_config.mqtt.broker_host,
        app_config.mqtt.broker_port,
    );
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_max_packet_size(MQTT_MAX_PACKET_SIZE, MQTT_MAX_PACKET_SIZE);
    info!("Starting MQTT server with options {:?}", mqttoptions);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

    let base_topic = app_config.mqtt.base_route;

    info!("MQTT base topic {}", base_topic);

    tokio::spawn({
        let mqtt_client = client.clone();
        let topic_v1 = format!("{base_topic}/new_message/v1");
        let topic_serenity_format_v1 = format!("{base_topic}/new_message/serenity_format/v1");
        async move {
            loop {
                let message = discord_message_receiver.recv().await.unwrap();
                // send using my own converted message format
                let mqtt_payload: ReceivedDiscordMessage = message.clone().into();
                let json =
                    serde_json::to_string(&mqtt_payload).expect("Failed to serialize message");
                if let Err(e) = mqtt_client
                    .publish(&topic_v1, QoS::AtMostOnce, false, json)
                    .await
                {
                    error!("Failed sending mqtt message {e}");
                }

                // send using the serenity message format
                let serenity_format_json =
                    serde_json::to_string(&message).expect("Failed to serialize message");
                if let Err(e) = mqtt_client
                    .publish(
                        &topic_serenity_format_v1,
                        QoS::AtMostOnce,
                        false,
                        serenity_format_json,
                    )
                    .await
                {
                    error!("Failed sending mqtt message {e}");
                }
            }
        }
    });

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
                DoorSensorHandler::new(discord_http.clone(), app_config.home.spam_channel_id),
            )
            .unwrap();

        router
            .add_handler(
                "zigbee2mqtt/switch/#",
                SwitchHandler::new(discord_http.clone(), app_config.home.spam_channel_id),
            )
            .unwrap();

        router
            .add_handler(
                "zigbee2mqtt/motion/#",
                MotionSensorHandler::new(discord_http.clone(), app_config.home.spam_channel_id),
            )
            .unwrap();

        router
            .add_handler(
                &format!("{base_topic}/say_channel"),
                DiscordChannelMessageHandler::new(discord_http.clone()),
            )
            .unwrap();

        router
            .add_handler(
                &format!("{base_topic}/send_file_channel"),
                DiscordChannelFileMessageHandler::new(discord_http.clone()),
            )
            .unwrap();

        router
            .add_handler(
                &format!("{base_topic}/show_typing_channel"),
                DiscordChannelShowTypingHandler::new(discord_http.clone()),
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

/// Simplified representation of message for use over mqtt
#[derive(Debug, Serialize)]
struct ReceivedDiscordMessage {
    message_id: u64,
    author_id: u64,
    is_author_bot: bool,
    channel_id: u64,
    content: String,
    attachments: Vec<MessageAttachment>,
}

impl From<Message> for ReceivedDiscordMessage {
    fn from(message: Message) -> Self {
        Self {
            message_id: message.id.0,
            author_id: message.author.id.0,
            is_author_bot: message.author.bot,
            channel_id: message.channel_id.0,
            content: message.content,
            attachments: message
                .attachments
                .into_iter()
                .map(MessageAttachment::from)
                .collect(),
        }
    }
}

#[derive(Debug, Serialize)]
struct MessageAttachment {
    id: u64,
    filename: String,
    height: Option<u64>,
    proxy_url: String,
    size: u64,
    url: String,
    width: Option<u64>,
    content_type: Option<String>,
    ephemeral: bool,
}

impl From<Attachment> for MessageAttachment {
    fn from(attachment: Attachment) -> Self {
        Self {
            id: attachment.id.0,
            filename: attachment.filename,
            height: attachment.height,
            proxy_url: attachment.proxy_url,
            size: attachment.size,
            url: attachment.url,
            width: attachment.width,
            content_type: attachment.content_type,
            ephemeral: attachment.ephemeral,
        }
    }
}
