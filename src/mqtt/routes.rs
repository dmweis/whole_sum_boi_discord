use super::router::RouteHandler;

use async_trait::async_trait;
use log::*;
use serde::Deserialize;
use std::{str::from_utf8, sync::Arc};
use tokio::sync::Mutex;

pub struct SimpleHandler {}

impl SimpleHandler {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

#[async_trait]
impl RouteHandler for SimpleHandler {
    async fn call(&mut self, _topic: &str, content: &[u8]) -> anyhow::Result<()> {
        let command: JsonData = serde_json::from_slice(content)?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct JsonData {
    content: String,
}
