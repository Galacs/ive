mod commands;
mod flows;
mod utils;

use std::env;
use std::path::Path;

use models::InteractionError;
use serenity::async_trait;
use serenity::model::application::interaction::Interaction;
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::model::prelude::command::CommandType;
use serenity::prelude::*;
use tokio::fs::create_dir;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            // println!("Received command interaction: {:#?}", command);

            let result = match command.data.name.as_str() {
                "ping" => commands::ping::run(&command, &ctx).await,
                "Edit video" => commands::edit::run(&command, &ctx).await,
                _ => Err(InteractionError::NotImplemented),
            };
            if let Err(why) = result {
                println!("{:?}", why);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        let guild_id = GuildId(
            env::var("GUILD_ID")
                .expect("Expected GUILD_ID in environment")
                .parse()
                .expect("GUILD_ID must be an integer"),
        );

        let _commands = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands
                .create_application_command(|command| commands::ping::register(command))
                .create_application_command(|command| {
                    commands::edit::register(command).kind(CommandType::Message)
                })
        })
        .await;

        // println!("I now have the following guild slash commands: {:#?}", _commands);

        // let guild_command = Command::create_global_application_command(&ctx.http, |command| {
        //     commands::ping::register(command)
        // })
        // .await;
        // println!("I created the following global slash command: {:#?}", guild_command);
    }
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Create tmp folder
    if !Path::new("tmpfs").exists() {
        if let Err(why) = create_dir("tmpfs").await {
            panic!("Can't create tmp dir: {}", why);
        }
    }

    // Build our client.
    let mut client = Client::builder(token, GatewayIntents::empty())
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
