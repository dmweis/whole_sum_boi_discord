use super::router::RouteHandler;
use async_trait::async_trait;
use log::*;
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
    async fn call(&mut self, _topic: &str, content: &[u8]) -> anyhow::Result<()> {
        info!("Handling door sensor data");
        let door_sensor: DoorSensor = serde_json::from_slice(content)?;

        let channel = ChannelId(self.discord_channel_id);
        if door_sensor.contact {
            channel.say(&self.discord, "Front door was closed").await?;
        } else {
            channel.say(&self.discord, "Front door was opened").await?;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct DoorSensor {
    #[allow(dead_code)]
    pub battery: i64,
    #[allow(dead_code)]
    pub battery_low: bool,
    pub contact: bool,
    #[allow(dead_code)]
    pub linkquality: i64,
    #[allow(dead_code)]
    pub tamper: bool,
    #[allow(dead_code)]
    pub voltage: i64,
}

pub struct MotionSensorHandler {
    discord: Arc<Http>,
    discord_channel_id: u64,
}

impl MotionSensorHandler {
    pub fn new(discord: Arc<Http>, discord_channel_id: u64) -> Box<Self> {
        Box::new(Self {
            discord,
            discord_channel_id,
        })
    }
}

#[async_trait]
impl RouteHandler for MotionSensorHandler {
    async fn call(&mut self, _topic: &str, content: &[u8]) -> anyhow::Result<()> {
        info!("Handling motion sensor data");
        let motion_sensor: MotionSensorData = serde_json::from_slice(content)?;

        let channel = ChannelId(self.discord_channel_id);
        if motion_sensor.occupancy {
            channel
                .say(&self.discord, "Motion sensor detected motion")
                .await?;
        } else {
            channel
                .say(&self.discord, "Motion sensors not detecting any motion")
                .await?;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct MotionSensorData {
    #[allow(dead_code)]
    pub battery: i64,
    #[allow(dead_code)]
    pub battery_low: bool,
    #[allow(dead_code)]
    pub linkquality: i64,
    pub occupancy: bool,
    #[allow(dead_code)]
    pub tamper: bool,
    #[allow(dead_code)]
    pub voltage: i64,
}
