use std::path::Path;
use std::time::Duration;

use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::prelude::Context;

use tokio::fs::{self, remove_dir_all, File};
use tokio::io::AsyncWriteExt;

use crate::flows;
use models::{EditError, InteractionError};

use models::Job;

pub async fn run(
    cmd: &ApplicationCommandInteraction,
    ctx: &Context,
) -> Result<(), InteractionError> {
    // Get message the command was called on
    let message = &cmd
        .data
        .resolved
        .messages
        .iter()
        .next()
        .ok_or(InteractionError::Error)?
        .1;

    // Check if the message contains a valid number of attachments
    let number_of_files = message.attachments.len();
    if number_of_files != 1 {
        cmd.create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.content("Le message doit contenir exlusivement une seule vidéo")
                })
        })
        .await?;
        return Err(crate::InteractionError::Edit(EditError::WrongFileNumber(
            number_of_files as u32,
        )));
    }

    // Create interaction response asking what edit to apply
    cmd.create_interaction_response(&ctx.http, |response| {
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
    .await?;
    // Get message of interaction reponse
    let interaction_reponse = &cmd.get_interaction_response(&ctx.http).await?;

    // Await edit apply choice (with timeout)
    let cmd = match interaction_reponse
        .await_component_interaction(&ctx)
        .timeout(Duration::from_secs(60 * 3))
        .await
    {
        Some(x) => x,
        None => {
            cmd.edit_original_interaction_response(&ctx.http, |response| {
                response
                    .content("T trop lent, j'ai pas ton temps")
                    .components(|comp| comp)
            })
            .await?;
            return Err(InteractionError::Timeout);
        }
    };

    // Get edit kind from awaited interaction
    let edit_kind = &cmd.data.values[0].to_owned();

    // Match edit kinds
    let info = match edit_kind.as_str() {
        "encode_to_size" => flows::encode_to_size::get_info(&cmd, &ctx, message).await?,
        _ => {
            return Err(InteractionError::InvalidInput(
                models::InvalidInputError::Error,
            ))
        }
    };

    // Notify file download
    cmd.edit_original_interaction_response(&ctx, |r| {
        r.content(format!(
            "Telechargement de **{}**...",
            message.attachments[0].filename
        ))
        .components(|comp| comp)
    })
    .await?;

    // Await file download
    let file = message.attachments[0].download().await?;

    // Define working directory and destination filepath
    let dir = Path::new("tmpfs").join(format!("{}", cmd.token));
    let dir = std::env::current_dir()?.join(dir);
    let dest_file = dir.join(&format!("edit-{}", message.attachments[0].filename));

    // Creating working directory
    fs::create_dir(&dir).await?;
    let path = dir.join(&message.attachments[0].filename);

    // Writing file to disk
    let mut buffer = File::create(&path).await?;
    buffer.write_all(&file).await?;

    // Notify file editing
    cmd.edit_original_interaction_response(&ctx.http, |response| {
        response
            .content(format!(
                "Modification de **{}**...",
                message.attachments[0].filename
            ))
            .components(|comp| comp)
    })
    .await?;

    // Edit file
    match info {
        Job::EncodeToSize(_, params) => {
            flows::encode_to_size::run(&path, dest_file.to_str().unwrap_or_default(), params).await
        }
    }

    // Notify file upload
    cmd.edit_original_interaction_response(&ctx.http, |response| {
        response
            .content(format!(
                "Envoi de **{}** modifié...",
                message.attachments[0].filename
            ))
            .components(|comp| comp)
    })
    .await?;

    // Upload files (sends message)
    let paths = vec![dest_file.to_str().unwrap_or_default()];

    cmd.channel_id
        .send_files(&ctx.http, paths, |m| {
            m.content(format!("**{}**:", message.attachments[0].filename))
        })
        .await?;

    // Delete working dir
    remove_dir_all(dir).await?;

    // Edit original interaction to notify sucess
    cmd
        .edit_original_interaction_response(&ctx.http, |response| {
            response
                .content(format!(
                    "**{}** à été modifié avec success",
                    message.attachments[0].filename
                ))
                .components(|comp| comp)
        })
        .await?;

    Ok(())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("Edit video")
}
