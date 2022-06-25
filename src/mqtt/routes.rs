use async_trait::async_trait;
use log::*;
use mqtt_router::{RouteHandler, RouterError};
use serde::Deserialize;
use serenity::{http::Http, model::id::ChannelId};
use std::sync::Arc;

pub struct DoorSensorHandler {
    discord: Arc<Http>,
    discord_channel_id: u64,
}

impl DoorSensorHandler {
    pub fn new(discord: Arc<Http>, discord_channel_id: u64) -> Box<Self> {
        Box::new(Self {
            discord,
            discord_channel_id,
        })
    }
}

#[async_trait]
impl RouteHandler for DoorSensorHandler {
    async fn call(&mut self, _topic: &str, content: &[u8]) -> std::result::Result<(), RouterError> {
        info!("Handling door sensor data");
        let door_sensor: DoorSensor =
            serde_json::from_slice(content).map_err(|e| RouterError::HandlerError(e.into()))?;

        let channel = ChannelId(self.discord_channel_id);
        if door_sensor.contact {
            channel
                .say(&self.discord, "Front door was closed")
                .await
                .map_err(|e| RouterError::HandlerError(e.into()))?;
        } else {
            channel
                .say(&self.discord, "Front door was opened")
                .await
                .map_err(|e| RouterError::HandlerError(e.into()))?;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct DoorSensor {
    #[allow(dead_code)]
    pub battery: f32,
    #[allow(dead_code)]
    pub battery_low: bool,
    pub contact: bool,
    #[allow(dead_code)]
    pub linkquality: f32,
    #[allow(dead_code)]
    pub tamper: bool,
    #[allow(dead_code)]
    pub voltage: f32,
}

pub struct MotionSensorHandler {
    discord: Arc<Http>,
    discord_channel_id: u64,
}

impl MotionSensorHandler {
    #[allow(dead_code)]
    pub fn new(discord: Arc<Http>, discord_channel_id: u64) -> Box<Self> {
        Box::new(Self {
            discord,
            discord_channel_id,
        })
    }
}

#[async_trait]
impl RouteHandler for MotionSensorHandler {
    async fn call(&mut self, _topic: &str, content: &[u8]) -> std::result::Result<(), RouterError> {
        info!("Handling motion sensor data");
        let motion_sensor: MotionSensorData =
            serde_json::from_slice(content).map_err(|e| RouterError::HandlerError(e.into()))?;

        let channel = ChannelId(self.discord_channel_id);
        if motion_sensor.occupancy {
            channel
                .say(&self.discord, "Motion sensor detected motion")
                .await
                .map_err(|e| RouterError::HandlerError(e.into()))?;
        } else {
            channel
                .say(&self.discord, "Motion sensors not detecting any motion")
                .await
                .map_err(|e| RouterError::HandlerError(e.into()))?;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct MotionSensorData {
    #[allow(dead_code)]
    pub battery: f32,
    #[allow(dead_code)]
    pub battery_low: bool,
    #[allow(dead_code)]
    pub linkquality: f32,
    pub occupancy: bool,
    #[allow(dead_code)]
    pub tamper: bool,
    #[allow(dead_code)]
    pub voltage: f32,
}

pub struct SwitchHandler {
    discord: Arc<Http>,
    discord_channel_id: u64,
}

impl SwitchHandler {
    pub fn new(discord: Arc<Http>, discord_channel_id: u64) -> Box<Self> {
        Box::new(Self {
            discord,
            discord_channel_id,
        })
    }
}

#[async_trait]
impl RouteHandler for SwitchHandler {
    async fn call(&mut self, topic: &str, content: &[u8]) -> std::result::Result<(), RouterError> {
        info!("Handling switch data");
        let switch_name = topic.split('/').last().unwrap_or("unknown");
        let switch_data: SwitchPayload =
            serde_json::from_slice(content).map_err(|err| RouterError::HandlerError(err.into()))?;

        let message = match switch_data.action {
            Action::Single => format!("switch {switch_name} was clicked once"),
            Action::Long => format!("switch {switch_name} was long pressed"),
            Action::Double => format!("switch {switch_name} was double clicked"),
        };

        let channel = ChannelId(self.discord_channel_id);
        channel
            .say(&self.discord, &message)
            .await
            .map_err(|e| RouterError::HandlerError(e.into()))?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Single,
    Double,
    Long,
}

#[derive(Debug, Deserialize)]
pub struct SwitchPayload {
    pub action: Action,
    #[allow(dead_code)]
    pub battery: f32,
    #[allow(dead_code)]
    pub linkquality: f32,
    #[allow(dead_code)]
    pub voltage: f32,
}
