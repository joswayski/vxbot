use std::env;

use dashmap::DashMap;
use dotenvy::dotenv;
use regex::Regex;
use serenity::all::{ChannelId, CreateAllowedMentions, CreateWebhook, ExecuteWebhook};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::webhook::Webhook;
use serenity::prelude::*;
struct Handler;

const VXTWITTER: &'static str = "vxtwitter.com";
const VXBOT: &'static str = "vxbot";

struct WebhookCache;
impl TypeMapKey for WebhookCache {
    type Value = DashMap<ChannelId, Webhook>;
}

async fn get_webhook_for_channel(
    ctx: &Context,
    channel_id: ChannelId,
) -> Result<Webhook, SerenityError> {
    let data = ctx.data.read().await;
    let cache: &DashMap<ChannelId, Webhook> = data
        .get::<WebhookCache>()
        .ok_or(SerenityError::Other("WebhookCache not found"))?;

    if let Some(webhook) = cache.get(&channel_id) {
        return Ok(webhook.clone());
    }

    let webhooks = channel_id.webhooks(&ctx.http).await?;

    if let Some(webhook) = webhooks
        .into_iter()
        .find(|wh| wh.name.as_deref() == Some(VXBOT))
    {
        cache.insert(channel_id, webhook.clone());
        return Ok(webhook);
    }

    let new_webhook = channel_id
        .create_webhook(&ctx.http, CreateWebhook::new(VXBOT))
        .await?;
    cache.insert(channel_id, new_webhook.clone());

    Ok(new_webhook)
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot || msg.author.name == VXBOT {
            // Easy skip
            return;
        }

        // Regex to match twitter.com or x.com URLs (with optional subdomains)
        // Matches: https://twitter.com/..., http://x.com/..., https://mobile.twitter.com/..., etc.
        // Specifically matches 'twitter' or 'x' as the domain name to avoid matching "phoronix.com"
        let re = Regex::new(r"https?://(?:([a-zA-Z0-9-]+)\.)?(twitter|x)\.com(/[^\s]*)?").unwrap();

        // Check if there are any twitter/x URLs
        if !re.is_match(&msg.content) {
            return;
        }

        // Skip if already converted to vxtwitter
        if msg.content.contains(VXTWITTER) {
            return;
        }

        // Replace twitter.com and x.com domains with vxtwitter.com
        let new_msg = re
            .replace_all(&msg.content, |caps: &regex::Captures| {
                // Capture group 3 is the path (group 1 is subdomain, group 2 is twitter|x)
                format!(
                    "https://{}{}",
                    VXTWITTER,
                    caps.get(3).map_or("", |m| m.as_str())
                )
            })
            .to_string();

        let webhook = match get_webhook_for_channel(&ctx, msg.channel_id).await {
            Ok(webhook) => webhook,
            Err(why) => {
                println!("Error getting webhooks for channel {why:?}");
                return;
            }
        };

        // Use the member's display name (nickname if set, otherwise username)
        let display_name = msg
            .member
            .as_ref()
            .and_then(|m| m.nick.as_deref())
            .unwrap_or(&msg.author.name);

        let builder = ExecuteWebhook::new()
            .content(new_msg)
            .username(display_name)
            .allowed_mentions(CreateAllowedMentions::new().empty_users().empty_roles())
            .avatar_url(msg.author.face());

        if let Err(why) = webhook.execute(&ctx.http, false, builder).await {
            println!("Error ocurred poasting message {why:?}");
            return;
        }

        // Delete only after the new message is posted
        if let Err(why) = msg.delete(&ctx.http).await {
            println!("Error deleting previous message {why:?}");
            return;
        }
    }
}

#[tokio::main]
async fn main() {
    eprintln!("Starting vxbot...");
    dotenv().ok();

    eprintln!("Loading DISCORD_TOKEN from environment...");
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    eprintln!("Token loaded successfully");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_WEBHOOKS;

    eprintln!("Creating client...");
    let mut client = match Client::builder(&token, intents)
        .event_handler(Handler)
        .type_map_insert::<WebhookCache>(DashMap::new())
        .await
    {
        Ok(client) => client,
        Err(e) => {
            eprintln!("Error creating client: {e:?}");
            return;
        }
    };

    eprintln!("Starting client...");
    if let Err(why) = client.start().await {
        eprintln!("Client error: {why:?}");
    }
    eprintln!("Client stopped");
}
