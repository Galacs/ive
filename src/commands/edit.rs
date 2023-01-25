use std::path::{Path, PathBuf};
use std::time::Duration;

use serenity::async_trait;
use serenity::builder::CreateApplicationCommand;
use serenity::futures::StreamExt;
use serenity::model::prelude::Message;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::interaction::message_component::MessageComponentInteraction;
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::prelude::Context;

use crate::flows;
use models::{EditError, InteractionError, JobProgress, Video};

use models::Job;
use queue::Queue;

#[async_trait]
pub trait EditMessage {
    async fn edit(&self, http: &serenity::http::Http, message: &str) -> Result<(), InteractionError>;
}

#[async_trait]
impl EditMessage for MessageComponentInteraction {
    async fn edit(&self, http: &serenity::http::Http, message: &str) -> Result<(), InteractionError> {
        self.edit_original_interaction_response(http.as_ref(), |r| {
            r.content(message).components(|comp| comp)
        })
        .await?;
        Ok(())
    }
}

#[async_trait]
impl EditMessage for ApplicationCommandInteraction {
    async fn edit(&self, http: &serenity::http::Http, message: &str) -> Result<(), InteractionError> {
        self.edit_original_interaction_response(http.as_ref(), |r| {
            r.content(message).components(|comp| comp)
        })
        .await?;
        Ok(())
    }
}

pub trait GetMessage {
    fn get_message(&self) -> Result<&Message, InteractionError>;
}

impl GetMessage for ApplicationCommandInteraction {
    fn get_message(&self) -> Result<&Message, InteractionError> {
        Ok(self
        .data
        .resolved
        .messages
        .iter()
        .next()
        .ok_or(InteractionError::Error)?
        .1)
    }
}



pub async fn run(
    cmd: &ApplicationCommandInteraction,
    ctx: &Context,
) -> Result<(), InteractionError> {
    // Get message the command was called on
    let message = cmd.get_message()?;
    let id = cmd.token.to_owned();

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
                                    o.label("Changer la taille du fichier (Preview)")
                                        .value("encode_to_size")
                                });
                                f.create_option(|o| {
                                    o.label("Couper la video (Preview)")
                                        .value("cut")
                                });
                                f.create_option(|o| {
                                    o.label("Changer le container (Preview)")
                                        .value("remux")
                                });
                                f.create_option(|o| {
                                    o.label("Combiner des medias (Preview)")
                                        .value("combine")
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
    let Some(interaction_reponse) = interaction_reponse
        .await_component_interaction(&ctx)
        .timeout(Duration::from_secs(60 * 3))
        .await else {
        cmd.edit(&ctx.http, "T trop lent, j'ai pas ton temps").await?;
        return Err(InteractionError::Timeout);
    };

    // Get edit kind from awaited interaction
    let edit_kind = &interaction_reponse.data.values[0].to_owned();

    // Match edit kinds
    let params = match edit_kind.as_str() {
        "encode_to_size" => flows::encode_to_size::get_info(&cmd, &interaction_reponse, &ctx).await,
        "cut" => flows::cut::get_info(&cmd, &interaction_reponse, &ctx).await,
        "remux" => flows::remux::get_info(&cmd, &interaction_reponse, &ctx).await,
        "combine" => flows::combine::get_info(&cmd, &interaction_reponse, &ctx).await,
        _ => {
            return Err(InteractionError::InvalidInput(
                models::InvalidInputError::Error,
            ))
        }
    };

    let Ok(params) = params else {
        return Ok(())
    };

    // let error_message;
    // let params = match params {
    //     Err(err) => {
    //         error_message = match err {
    //             _ => "salut",
    //         };
    //         edit_interaction(&cmd, &ctx, error_message).await?;
    //         return Err(err);
    //     }
    //     Ok(p) => p,
    // };

    // Notify file download
    cmd.edit(&ctx.http, &format!("Telechargement de **{}**...",  message.attachments[0].filename)).await?;

    // Notify file queuing
    cmd.edit(&ctx.http, &format!("**{}** à été mit dans la file d'attente", message.attachments[0].filename)).await?;

    let attachment = message.attachments[0].clone();

    let client = config::get_redis_client();
    let mut con = client.get_async_connection().await?;

    // Build job obj
    let video = Video::new(
        models::VideoURI::Url(attachment.url),
        Some(id.to_owned()),
        attachment.filename.to_owned(),
    );
    let job = Job::new(models::JobKind::Processing, Some(video), params);

    // Send job to redis queue
    job.send_job(&mut con).await?;

    // Subscribe to status queue
    let mut pubsub = client.get_async_connection().await?.into_pubsub();
    let channel = format!("progress:{}", id);
    pubsub.subscribe(&channel).await?;
    let mut msg_stream = pubsub.into_on_message();

    let extension;

    // Wait for done message
    loop {
        let payload: String = msg_stream
            .next()
            .await
            .ok_or(InteractionError::Error)?
            .get_payload()?;
        let progress: JobProgress = serde_json::from_str(&payload.as_str())?;
        match progress {
            JobProgress::Started => {
                println!("Starting conversion...");
                // Notify file queuing
                cmd.edit(&ctx.http, &format!("Modification de **{}**...", message.attachments[0].filename)).await?;
            }
            JobProgress::Done(fe) => {
                extension = fe;
                break
            },
            JobProgress::Progress(_) => todo!(),
            JobProgress::Error(err) => {
                    println!("Erreur du worker: {:?}", err);
                    return Err(InteractionError::Error);
            },
            JobProgress::Response(_) => todo!(),
        }
    }

    let bucket = config::get_s3_bucket();
    let res_files = bucket.get_object(&id).await?;
    bucket.delete_object(id).await?;
    let filesize = res_files.bytes().len();
    
    if filesize > (8 * 2_i32.pow(20)) as usize {
        cmd.edit(&ctx.http, &format!("**{}** ne peut pas être envoyé car {:.2}Mo > 8Mo (limite de discord)", message.attachments[0].filename, filesize / 2_usize.pow(20))).await?;
        return Ok(())
    }

    // Notify file upload
    cmd.edit(&ctx.http, &format!("Envoi de **{}** modifié...", message.attachments[0].filename)).await?;

    let mut path = PathBuf::new();
    path = path.join(Path::new(&attachment.filename));
    path.set_extension(extension);
    let filename = path.to_str().ok_or(InteractionError::Error)?;

    cmd.channel_id
        .send_message(&ctx.http, |m| {
            m.content(format!("**{}**:", message.attachments[0].filename));
            m.files(vec![(res_files.bytes(), filename)])
        })
        .await?;


    // Edit original interaction to notify sucess
    cmd.edit(&ctx.http, &format!("**{}** à été modifié avec success", message.attachments[0].filename)).await?;

    Ok(())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("Edit video")
}
