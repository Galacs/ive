use std::{collections::HashMap, time::Duration};

use uuid::Uuid;

use queue::Queue;
use serenity::{
    futures::StreamExt,
    model::prelude::{
        interaction::{
            application_command::ApplicationCommandInteraction,
            message_component::MessageComponentInteraction,
        },
        Attachment,
    },
    prelude::Context,
};

use models::{error, job, CombineParameters, CombineVideo, MediaStream, StreamKind, Video};

use crate::commands::edit::{EditMessage, GetMessage};

async fn get_streams(
    attachment: &Attachment,
    id: &str,
    cmd: &ApplicationCommandInteraction,
    ctx: &Context,
) -> Result<Vec<MediaStream>, error::Interaction> {
    let client = config::get_redis_client();
    let mut con = client.get_async_connection().await?;

    // Build job obj
    let video = Video::new(
        models::VideoURI::Url(attachment.url.to_owned()),
        Some(id.to_owned()),
        attachment.filename.to_owned(),
    );
    let job = job::Job::new(job::Kind::Parsing, Some(video), job::Parameters::GetStreams);

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
            .ok_or(error::Interaction::Error)?
            .get_payload()?;
        let progress: job::Progress = serde_json::from_str(&payload.as_str())?;
        match progress {
            job::Progress::Started => {
                cmd.edit(
                    &ctx.http,
                    &format!("Analyse de **{}**...", attachment.filename),
                )
                .await?;
            }
            job::Progress::Error(err) => {
                println!("Erreur du worker: {:?}", err);
                return Err(error::Interaction::Error);
            }
            job::Progress::Response(res) => match res {
                job::Response::GetStreams(res) => return Ok(res),
            },
            _ => {}
        }
    }
}

async fn update_msg(
    _attachment: &Attachment,
    cmd: &ApplicationCommandInteraction,
    ctx: &Context,
    streams: &Vec<StreamState>,
) -> Result<(), error::Interaction> {
    fn get_name(stream: &MediaStream) -> &str {
        match stream.kind {
            models::StreamKind::Video => "Video",
            models::StreamKind::Audio => "Audio",
            models::StreamKind::Unknown => "toz",
        }
    }
    cmd.edit_original_interaction_response(&ctx.http, |m| {
        let mut streams_str = String::new();
        for stream in streams {
            let status = match stream.is_kept {
                Some(s) => match s {
                    true => "gardé",
                    false => "retiré",
                },
                None => "pas encore choisi",
            };
            let name = get_name(&stream.stream);
            streams_str.push_str(&format!(
                "{}: {}\n",
                format!("{} {}", name, stream.filename),
                status
            ))
        }
        m.content(streams_str);
        m.components(|comps| {
            for stream in streams {
                if stream.is_selected {
                    continue;
                }
                let name = get_name(&stream.stream);
                comps.create_action_row(|row| {
                    row.create_select_menu(|m| {
                        m.custom_id(stream.uuid);
                        m.placeholder(format!("{} {}", name, stream.filename));
                        m.options(|f| {
                            f.create_option(|o| {
                                o.label("Garder dans le media final").value("keep")
                            });
                            f.create_option(|o| o.label("Retirer du media final").value("exlude"))
                        })
                    })
                });
            }
            if streams.iter().all(|x| x.is_selected) {
                comps.create_action_row(|r| {
                    r.create_button(|b| {
                        b.custom_id("confirm");
                        b.label("Feur")
                    })
                });
            }

            comps
        })
    })
    .await?;
    Ok(())
}

#[derive(Debug)]
struct StreamState {
    uuid: Uuid,
    filename: String,
    url: String,
    stream: MediaStream,
    is_selected: bool,
    is_kept: Option<bool>,
}

pub async fn get_info(
    cmd: &ApplicationCommandInteraction,
    interaction_reponse: &MessageComponentInteraction,
    ctx: &Context,
) -> Result<job::Parameters, error::Interaction> {
    // Create interaction response asking what edit to apply
    interaction_reponse.defer(&ctx.http).await?;

    let sender_message = cmd.get_message()?;

    cmd.edit_original_interaction_response(&ctx.http, |m| {
        m.content(format!(
            "**{}** en attente...",
            sender_message.attachments[0].filename
        ));
        m.components(|c| c)
    })
    .await?;

    let mut streams: Vec<StreamState> =
        get_streams(&sender_message.attachments[0], "crienclarue", &cmd, &ctx)
            .await?
            .into_iter()
            .map(|x| StreamState {
                uuid: Uuid::new_v4(),
                filename: sender_message.attachments[0].filename.to_owned(),
                stream: x,
                is_selected: false,
                is_kept: None,
                url: sender_message.attachments[0].url.to_owned(),
            })
            .collect();

    loop {
        if let Err(err) = update_msg(&sender_message.attachments[0], &cmd, &ctx, &streams).await {
            println!("Erreur de mise a jour: {:?}", err);
        }

        // Await edit apply choice (with timeout)
        let interaction_reponse = &cmd.get_interaction_response(&ctx.http).await?;
        let interaction_id = cmd.id;

        tokio::select! {
            i = interaction_reponse.await_component_interaction(&ctx).timeout(Duration::from_secs(60 * 3)) => {
                let interaction = i.unwrap();
                if let Err(e) = interaction.defer(&ctx.http).await {
                    println!("{e}");
                }

                if interaction.data.custom_id == "confirm" {
                    break;
                }

                let uuid: Uuid = interaction.data.custom_id.parse().unwrap();
                let choice  = match interaction.data.values[0].as_str() {
                    "keep" => true,
                    "exlude" => false,
                    _ => false,
                };
                let index = streams.iter().position(|x| x.uuid == uuid).unwrap();
                let s = streams.get_mut(index).unwrap();
                s.is_selected = true;
                s.is_kept = Some(choice);
            },
            msg = cmd.user.await_reply(&ctx).filter(move |x| {
                x.referenced_message.as_ref().unwrap().interaction.as_ref().unwrap().id == interaction_id
            }) => {
                let s: Vec<StreamState> = get_streams(&msg.as_ref().unwrap().attachments[0], "crienclarue", &cmd, &ctx).await?.into_iter().map(|x| StreamState { uuid: Uuid::new_v4(), filename: msg.as_ref().unwrap().attachments[0].filename.to_owned(), stream: x, is_selected: false, is_kept: None, url: msg.as_ref().unwrap().attachments[0].url.to_owned() }).collect();
                streams.extend(s);
            }
        };
    }

    let mut hashmap: HashMap<String, CombineVideo> = HashMap::new();

    let mut kind: Option<StreamKind> = None;

    streams
        .into_iter()
        .filter(|x| x.is_kept.unwrap())
        .for_each(|x| {
            if !hashmap.contains_key(&x.url) {
                kind = match (&kind, &x.stream.kind) {
                    (None, StreamKind::Video) => Some(StreamKind::Video),
                    (None, StreamKind::Audio) => Some(StreamKind::Audio),
                    (Some(StreamKind::Video), StreamKind::Video) => Some(StreamKind::Video),
                    (Some(StreamKind::Video), StreamKind::Audio) => Some(StreamKind::Video),
                    (Some(StreamKind::Audio), StreamKind::Video) => Some(StreamKind::Video),
                    (Some(StreamKind::Audio), StreamKind::Audio) => Some(StreamKind::Audio),
                    _ => None,
                };
                hashmap.insert(
                    x.url.to_owned(),
                    CombineVideo {
                        url: x.url.to_owned(),
                        selected_streams: Vec::new(),
                    },
                );
            }
            hashmap
                .get_mut(&x.url)
                .unwrap()
                .selected_streams
                .push(x.stream.id);
        });

    let videos: Vec<CombineVideo> = hashmap.into_iter().map(|x| x.1).collect();
    Ok(job::Parameters::Combine(CombineParameters {
        videos,
        output_kind: kind.ok_or(error::Interaction::Error)?,
    }))
}
