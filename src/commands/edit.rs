use std::path::{PathBuf, Path};
use std::time::Duration;

use serenity::builder::CreateApplicationCommand;
use serenity::futures::StreamExt;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::prelude::Context;

use crate::flows;
use models::{EditError, InteractionError, Video};

use models::Job;
use queue::Queue;

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

    let id = cmd.token.to_owned();
    dbg!(&id);

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
    let params = match edit_kind.as_str() {
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

    let attachment = message.attachments[0].clone();

    let client = config::get_redis_client();
    let mut con = client.get_async_connection().await.unwrap();

    // Build job obj
    let video = Video::new(
        models::VideoURI::Url(attachment.url),
        Some(id.to_owned()),
        attachment.filename.to_owned(),
    );
    let job = Job::new(models::JobKind::EncodeToSize, Some(video), params);

    // Send job to redis queue
    job.send_job(&mut con).await.unwrap();

    let channel = format!("progress:{}", id);

    // Subscribe to status queue
    let mut pubsub = con.into_pubsub();

    pubsub.subscribe(&channel).await?;

    // Wait for done message
    loop {
        let mut pubsub = client.get_async_connection().await?.into_pubsub();
        pubsub.subscribe(&channel).await?;
        let mut message = pubsub.into_on_message();
        let payload: String = message.next().await.unwrap().get_payload()?;
        match payload.as_str() {
            "starting" => println!("Starting conversion..."),
            "done" => break,
            _ => {}
        }
    }

    let bucket = config::get_s3_bucket();
    let res_files = bucket.get_object(&id).await?;
    
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

    let mut path = PathBuf::new();
    path = path.join(Path::new(&attachment.filename));
    path.set_extension("mp4");
    let filename = path.to_str().ok_or(InteractionError::Error)?;

    cmd.channel_id
        .send_message(&ctx.http, |m| {
            m.content(format!("**{}**:", message.attachments[0].filename));
            m.files(vec![(
                res_files.bytes(),
                filename,
            )])
        })
        .await?;

    bucket.delete_object(id).await?;

    // Edit original interaction to notify sucess
    cmd.edit_original_interaction_response(&ctx.http, |response| {
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
