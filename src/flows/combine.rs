use std::{time::Duration, collections::{HashMap, HashSet}};

use queue::Queue;
use serenity::{
    futures::StreamExt,
    model::prelude::{
        interaction::{
            application_command::ApplicationCommandInteraction,
            message_component::MessageComponentInteraction,
        }, Attachment,
    },
    prelude::Context,
};

use models::{
    JobParameters, InteractionError, InvalidInputError,
    RemuxParameters, VideoContainer, MediaStream, Job, Video, JobProgress,
};

use crate::commands::edit::{EditMessage, GetMessage};

async fn get_streams(attachment: &Attachment, id: &str, cmd: &ApplicationCommandInteraction, ctx: &Context, ) -> Result<HashMap::<i32, MediaStream>, InteractionError> {

    let client = config::get_redis_client();
    let mut con = client.get_async_connection().await?;

    // Build job obj
    let video = Video::new(
        models::VideoURI::Url(attachment.url.to_owned()),
        Some(id.to_owned()),
        attachment.filename.to_owned(),
    );
    let job = Job::new(models::JobKind::Parsing, Some(video), JobParameters::GetStreams);

    // Send job to redis queue
    job.send_job(&mut con).await?;

    // Subscribe to status queue
    let mut pubsub = client.get_async_connection().await?.into_pubsub();
    let channel = format!("progress:{}", id);
    pubsub.subscribe(&channel).await?;
    let mut msg_stream = pubsub.into_on_message();
    
    // Wait for reponse
    loop {
        let payload: String = msg_stream
            .next()
            .await
            .ok_or(InteractionError::Error)?
            .get_payload()?;
        let progress: JobProgress = serde_json::from_str(&payload.as_str())?;
        match progress {
            JobProgress::Started => {
                cmd.edit(&ctx.http, &format!("Analyse de **{}**...", attachment.filename)).await?;
            }
            JobProgress::Error(err) => {
                    println!("Erreur du worker: {:?}", err);
                    return Err(InteractionError::Error);
            },
            JobProgress::Response(res) => match res {
                models::JobResponse::GetStreams(res) => return Ok(res),
            },
            _ => {}
        }
    }
}

async fn update_msg(attachment: &Attachment, cmd: &ApplicationCommandInteraction, ctx: &Context, streams: &HashMap::<i32, MediaStream>, selected_streams: &HashMap<i32, bool>) -> Result<(), InteractionError> {
    fn get_name(stream: &MediaStream) -> &str{
        match stream.kind {
            models::StreamKind::Video => "Video",
            models::StreamKind::Audio => "Audio",
            models::StreamKind::Unknown => "toz",
        }
    }
    cmd.edit_original_interaction_response(&ctx.http, |m| {
        let mut streams_str = String::new();
        for (id, stream) in streams {
            let status = match selected_streams.get(id) {
                Some(s) => match s {
                    true => "gardé",
                    false => "retiré",
                },
                None => "pas encore choisi",
            };
            let name = get_name(stream);
            streams_str.push_str(&format!("{}: {}\n", name, status))
        }
        m.content(format!(
            "**{}**:\n\
            {}",
            streams_str,
            attachment.filename
        ));
        m.components(|comps| {
            for (id, stream) in streams {
                if selected_streams.contains_key(id) {
                    continue;
                }
                let name = get_name(stream);
                comps.create_action_row(|row| {
                    row.create_select_menu(|m| {
                        m.custom_id(id);
                        m.placeholder(name);
                        m.options(|f| {
                            f.create_option(|o| {
                                o.label("Garder dans le media final").value("keep")
                            });
                            f.create_option(|o| {
                                o.label("Retirer du media final").value("exlude")
                            })
                        })
                        
                    })
                });
            }
            comps
        })
    }).await?;
    Ok(())
}


pub async fn get_info(
    cmd: &ApplicationCommandInteraction,
    interaction_reponse: &MessageComponentInteraction,
    ctx: &Context
) -> Result<JobParameters, InteractionError> {
    
    // Create interaction response asking what edit to apply
    interaction_reponse.defer(&ctx.http).await?;

    let sender_message = cmd.get_message()?;

    cmd.edit_original_interaction_response(&ctx.http, |m| {
        m.content(format!("**{}** en attente...",
            sender_message.attachments[0].filename
        ));
        m.components(|c| c)
    }).await?;

    let streams = get_streams(&sender_message.attachments[0], "crienclarue", &cmd, &ctx).await?;
    dbg!(&streams);


    let mut selected_streams = HashMap::new(); // Change to hashmap to store user inupt
    for _ in 0..streams.len()+1 {
        if let Err(err) = update_msg(&sender_message.attachments[0], &cmd, &ctx, &streams, &selected_streams).await {
            println!("Erreur de mise a jour: {:?}", err);
        }
    
        // Await edit apply choice (with timeout)
        let interaction_reponse = &cmd.get_interaction_response(&ctx.http).await?;

        let interaction_id = cmd.id;

        dbg!(&interaction_id);

        tokio::select! {
            i = interaction_reponse.await_component_interaction(&ctx).timeout(Duration::from_secs(60 * 3)) => {
                let interaction = i.unwrap();
                if let Err(e) = interaction.defer(&ctx.http).await {
                    println!("{e}");
                }
                let int: i32 = interaction.data.custom_id.parse().unwrap();
                let choice  = match interaction.data.values[0].as_str() {
                    "keep" => true,
                    "exlude" => false,
                    _ => false,
                };
                selected_streams.insert(int, choice);
                dbg!(&selected_streams);
            },
            msg = cmd.user.await_reply(&ctx).filter(move |x| {
                dbg!(x.referenced_message.as_ref().unwrap().interaction.as_ref().unwrap().id);
                x.referenced_message.as_ref().unwrap().interaction.as_ref().unwrap().id == interaction_id
            }) => {
                println!("adding video...");
                println!("{}", msg.unwrap().content);
            }
        };

        // let Some(interaction) = interaction_reponse
        //     .await_component_interaction(&ctx)
        //     .timeout(Duration::from_secs(60 * 3))
        //     .await else {
        //     cmd.edit(&ctx.http, "T trop lent, j'ai pas ton temps").await?;
        //     return Err(InteractionError::Timeout);
        // };


    }

    let a = cmd.get_message()?.author.await_replies(&ctx).build().next().await;
    dbg!(a);


    panic!();

    // // Get edit kind from awaited interaction
    // let container = match interaction.data.values[0].as_str() {
    //     "mp4" => VideoContainer::MP4,
    //     "mkv" => VideoContainer::MKV,
    //     _ => {
    //         return Err(models::InteractionError::InvalidInput(
    //             InvalidInputError::Error,
    //         ))
    //     }
    // };

    // Ok(JobParameters::Remux(RemuxParameters { container }))
}
