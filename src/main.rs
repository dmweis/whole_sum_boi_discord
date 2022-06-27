mod configuration;
mod mqtt;

use crate::{configuration::get_configuration, mqtt::start_mqtt_service};
use log::*;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    model::{channel::Message, gateway::Ready, id::ChannelId},
    prelude::*,
};
use simplelog::*;
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};

struct DiscordMessageHandler {
    message_sender: UnboundedSender<Message>,
}

impl DiscordMessageHandler {
    pub fn new(message_sender: UnboundedSender<Message>) -> Self {
        Self { message_sender }
    }
}

#[async_trait]
impl EventHandler for DiscordMessageHandler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content.to_ascii_lowercase() == "ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                error!("Error sending message: {:?}", why);
            }
        }
        self.message_sender.send(msg).unwrap();
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

#[derive(StructOpt, Debug)]
#[structopt(
    version = "0.1.0",
    author = "David M. Weis <dweis7@gmail.com>",
    about = "Discord bot called WholeSumBoi"
)]
struct Opts {
    #[structopt(long)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_logging();
    let opts = Opts::from_args();
    let app_config = get_configuration(opts.config)?;

    // discord time
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let (sender, receiver) = unbounded_channel();

    let mut client = Client::builder(&app_config.discord.token, intents)
        .event_handler(DiscordMessageHandler::new(sender))
        .await
        .expect("Err creating client");

    let http = client.cache_and_http.http.clone();

    let channel = ChannelId(app_config.home.notification_discord_channel);
    channel.say(&http, "WholeSumBoi is online").await?;

    start_mqtt_service(app_config, http, receiver)?;

    info!("Starting discord client");
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
    Ok(())
}

fn setup_logging() {
    // only allow logs from our crate
    // serenity is spammy
    let config = ConfigBuilder::new()
        .add_filter_allow_str("whole_sum_boi_discord")
        .build();

    if TermLogger::init(
        LevelFilter::Info,
        config.clone(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .is_err()
    {
        eprintln!("Failed to create term logger");
        if SimpleLogger::init(LevelFilter::Info, config).is_err() {
            eprintln!("Failed to create simple logger");
        }
    }
}
