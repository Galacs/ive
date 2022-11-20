use std::path::Path;
use std::time::Duration;

use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::prelude::Context;

use tokio::fs::{self, remove_dir_all, File};
use tokio::io::AsyncWriteExt;

use crate::flows;
use crate::flows::info::Info;

pub async fn run(cmd: &ApplicationCommandInteraction, ctx: &Context) -> Result<(), String> {
    // Get message the command was called on
    let message = &cmd.data.resolved.messages.iter().next().unwrap().1;

    // Check if the message contains a valid number of attachments
    if message.attachments.len() != 1 {
        let _ = cmd
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.content("Le message doit contenir exlusivement une seule vidéo")
                    })
            })
            .await;
        return Ok(());
    }

    // Create interaction response asking what edit to apply
    if let Err(why) = cmd
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| {
                    m.content(format!(
                        "Que voulez vous faire avec **{}**...",
                        message.attachments[0].filename
                    ));
                    m.components(|comps| {
                        comps.create_action_row(|row| {
                            row.create_select_menu(|menu| {
                                menu.custom_id("edit_kind");
                                menu.placeholder("Choisissez une modification");
                                menu.options(|f| {
                                    f.create_option(|o| {
                                        o.label("Changer la taille du fichier")
                                            .value("encode_to_size")
                                    })
                                })
                            })
                        })
                    })
                })
        })
        .await
    {
        println!("Cannot respond to application command: {}", why);
    }
    // Get message of interaction reponse
    let interaction_reponse = &cmd.get_interaction_response(&ctx.http).await.unwrap();

    // Await edit apply choice (with timeout)
    let cmd = match interaction_reponse
        .await_component_interaction(&ctx)
        .timeout(Duration::from_secs(60 * 3))
        .await
    {
        Some(x) => x,
        None => {
            if let Err(why) = cmd.edit_original_interaction_response(&ctx.http, |response| {
                response
                    .content("T trop lent, j'ai pas ton temps")
                    .components(|comp| comp)
            })
            .await {
                println!("Can't send timeout message: {}", why);
            };
            return Ok(());
        }
    };

    // Get edit kind from awaited interaction
    let edit_kind = &cmd.data.values[0].to_owned();

    // Match edit kinds
    let info = match edit_kind.as_str() {
        "encode_to_size" => flows::encode_to_size::get_info(&cmd, &ctx, message).await,
        _ => Err(()),
    };

    // Notify file download
    if let Err(why) = cmd
        .edit_original_interaction_response(&ctx, |r| {
            r.content(format!(
                "Telechargement de **{}**...",
                message.attachments[0].filename
            ))
            .components(|comp| comp)
        })
        .await
    {
        println!("Cannot respond to slash command: {}", why);
    }
    // Await file download
    let file = message.attachments[0].download().await.unwrap();

    // Define working directory and destination filepath
    let dir = Path::new("tmpfs").join(format!("{}", cmd.token));
    let dir = std::env::current_dir().unwrap().join(dir);
    let dest_file = dir.join(&format!("edit-{}", message.attachments[0].filename));

    // Creating working directory
    if let Err(why) = fs::create_dir(&dir).await {
        println!("Error creating directory: {}", why)
    }
    let path = dir.join(&message.attachments[0].filename);

    // Writing file to disk
    let mut buffer = File::create(&path).await.unwrap();
    if let Err(why) = buffer.write_all(&file).await {
        println!("Error saving file: {}", why)
    };

    // Notify file editing
    if let Err(why) = cmd
        .edit_original_interaction_response(&ctx.http, |response| {
            response
                .content(format!(
                    "Modification de **{}**...",
                    message.attachments[0].filename
                ))
                .components(|comp| comp)
        })
        .await
    {
        println!("Cannot respond to slash command: {}", why);
    }

    // Edit file
    match info {
        Ok(Info::EncodeToSize(t_size)) => {
            flows::encode_to_size::run(&path, dest_file.to_str().unwrap(), t_size).await
        }
        _ => {}
    }

    // Notify file upload
    if let Err(why) = cmd
        .edit_original_interaction_response(&ctx.http, |response| {
            response
                .content(format!(
                    "Envoi de **{}** modifié...",
                    message.attachments[0].filename
                ))
                .components(|comp| comp)
        })
        .await
    {
        println!("Cannot respond to slash command: {}", why);
    }

    // Upload files (sends message)
    let paths = vec![dest_file.to_str().unwrap()];

    if let Err(why) = cmd
        .channel_id
        .send_files(&ctx.http, paths, |m| {
            m.content(format!("**{}**:", message.attachments[0].filename))
        })
        .await
    {
        println!("Error uploading file: {}", why);
    }

    // Delete working dir
    if let Err(why) = remove_dir_all(dir).await {
        println!("Can't delete videos directory: {}", why);
    }

    // Edit original interaction to notify sucess
    if let Err(why) = cmd
        .edit_original_interaction_response(&ctx.http, |response| {
            response
                .content(format!(
                    "**{}** à été modifié avec success",
                    message.attachments[0].filename
                ))
                .components(|comp| comp)
        })
        .await
    {
        println!("Cannot respond to slash command: {}", why);
    }

    Ok(())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("Edit video")
}
