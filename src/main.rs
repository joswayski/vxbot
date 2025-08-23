use std::env;

use dotenvy::dotenv;
use futures::future;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;
struct Handler;

const TWITTER: &'static str = "twitter.com";
const X: &'static str = "x.com";
const VXTWITTER: &'static str = "vxtwitter.com";

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        let replaced_urls = msg
            .content
            .split(" ")
            .filter_map(|section| {
                if section.contains(TWITTER) && !section.contains(VXTWITTER) {
                    Some(section.replace(TWITTER, VXTWITTER))
                } else if section.contains(X) {
                    Some(section.replace(X, VXTWITTER))
                } else {
                    None
                }
            })
            .collect::<Vec<String>>();

        if !replaced_urls.is_empty() {
            let handles: Vec<_> = replaced_urls
                .iter()
                .map(|link| {
                    let http = ctx.http.clone();
                    let url = link.clone();
                    tokio::spawn(async move {
                        if let Err(why) = msg.channel_id.say(http, url).await {
                            println!("Error sending message: {why:?}");
                        }
                    })
                })
                .collect();

            future::join_all(handles).await;
        }
    }
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .await
        .expect("Err creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
