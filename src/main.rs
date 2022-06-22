mod configuration;
mod mqtt;

use crate::{configuration::get_configuration, mqtt::start_mqtt_service};
use log::*;
use serenity::{
    async_trait,
    client::{Context, EventHandler},
    http::Http,
    model::{channel::Message, gateway::Ready, id::ChannelId},
    prelude::*,
};
use simplelog::*;
use std::path::PathBuf;
use structopt::StructOpt;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.content == "!ping" {
            if let Err(why) = msg.channel_id.say(&ctx.http, "Pong!").await {
                error!("Error sending message: {:?}", why);
            }
        }
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

    let mut client = Client::builder(&app_config.discord.token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    let http = client.cache_and_http.http.clone();

    tokio::spawn(async move { start_mqtt_service(app_config, http) });

    info!("Starting discord client");
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
    Ok(())
}

fn setup_logging() {
    if TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .is_err()
    {
        eprintln!("Failed to create term logger");
        if SimpleLogger::init(LevelFilter::Info, Config::default()).is_err() {
            eprintln!("Failed to create simple logger");
        }
    }
}
