mod commands;
mod flows;
mod utils;

use std::env;

use models::error;
use serenity::async_trait;
use serenity::model::application::interaction::Interaction;
use serenity::model::gateway::Ready;
// use serenity::model::id::GuildId;
use serenity::model::prelude::command::{CommandType, Command};
use serenity::prelude::*;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            // println!("Received command interaction: {:#?}", command);

            let result = match command.data.name.as_str() {
                "ping" => commands::ping::run(&command, &ctx).await,
                "Edit video" => commands::edit::run(&command, &ctx).await,
                _ => Err(error::Interaction::NotImplemented),
            };
            if let Err(why) = result {
                let _ = command.edit_original_interaction_response(&ctx.http, |response| {
                    response
                        .content("Erreur de traitement.")
                        .components(|comp| comp)
                })
                .await;
                println!("{:?}", why);
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        // let guild_id = GuildId(
        //     env::var("IVE_GUILD_ID")
        //         .expect("Expected GUILD_ID in environment")
        //         .parse()
        //         .expect("GUILD_ID must be an integer"),
        // );

        // let _commands = GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
        //     commands
        //         .create_application_command(|command| commands::ping::register(command))
        //         .create_application_command(|command| {
        //             commands::edit::register(command).kind(CommandType::Message)
        //         })
        // })
        // .await;

        // println!("I now have the following guild slash commands: {:#?}", _commands);

        let _guild_command = Command::create_global_application_command(&ctx.http, |command| {
            commands::ping::register(command);
            commands::edit::register(command).kind(CommandType::Message)
        })
        .await;
        // println!("I created the following global slash command: {:#?}", guild_command);
    }
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("IVE_DISCORD_TOKEN").expect("Expected a token in the environment");

    // Build our client.
    let mut client = Client::builder(token, GatewayIntents::GUILD_MESSAGES)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
