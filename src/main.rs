use std::env;
use std::sync::LazyLock;

use dashmap::DashMap;
use dotenvy::dotenv;
use regex::Regex;
use serenity::all::{ChannelId, CreateAllowedMentions, CreateWebhook, ExecuteWebhook};
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::model::webhook::Webhook;
use serenity::prelude::*;

const VXTWITTER: &str = "vxtwitter.com";
const VXBOT: &str = "vxbot";
const FACEBED: &str = "facebed.com";
const INSTABED: &str = "eeinstagram.com";

// Static regexes - compiled once at first use
static TWITTER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    // Matches: https://twitter.com/..., http://x.com/..., https://mobile.twitter.com/..., etc.
    // Specifically matches 'twitter' or 'x' as the domain name to avoid matching "phoronix.com"
    Regex::new(r"https?://(?:([a-zA-Z0-9-]+)\.)?(twitter|x)\.com(/[^\s]*)?").unwrap()
});

static FACEBOOK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"https?://(?:[a-zA-Z0-9-]+\.)?facebook\.com(/[^\s]*)?").unwrap());

static INSTAGRAM_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"https?://(?:[a-zA-Z0-9-]+\.)?instagram\.com(/[^\s]*)?").unwrap());

struct Handler;

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

        let has_twitter = TWITTER_REGEX.is_match(&msg.content);
        let has_facebook = FACEBOOK_REGEX.is_match(&msg.content);
        let has_instagram = INSTAGRAM_REGEX.is_match(&msg.content);

        // If nothing to match, return
        if !has_twitter && !has_facebook && !has_instagram {
            return;
        }

        let mut new_msg = msg.content.clone();

        if has_twitter {
            new_msg = TWITTER_REGEX
                .replace_all(&new_msg, |caps: &regex::Captures| {
                    // Capture group 3 is the path (group 1 is subdomain, group 2 is twitter|x)
                    format!(
                        "https://{}{}",
                        VXTWITTER,
                        caps.get(3).map_or("", |m| m.as_str())
                    )
                })
                .to_string();
        }
        if has_facebook {
            new_msg = FACEBOOK_REGEX
                .replace_all(&new_msg, |caps: &regex::Captures| {
                    format!(
                        "https://{}{}",
                        FACEBED,
                        caps.get(1).map_or("", |m| m.as_str())
                    )
                })
                .to_string();
        }
        if has_instagram {
            new_msg = INSTAGRAM_REGEX
                .replace_all(&new_msg, |caps: &regex::Captures| {
                    format!(
                        "https://{}{}",
                        INSTABED,
                        caps.get(1).map_or("", |m| m.as_str())
                    )
                })
                .to_string();
        }

        let webhook = match get_webhook_for_channel(&ctx, msg.channel_id).await {
            Ok(webhook) => webhook,
            Err(why) => {
                println!("Error getting webhooks for channel {why:?}");
                return;
            }
        };

        let display_name = msg
            .author_nick(&ctx.http)
            .await
            .or(msg.author.global_name.clone())
            .unwrap_or_else(|| msg.author.name.clone());

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
