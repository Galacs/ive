use std::time::Duration;

use serenity::{
    model::prelude::{
        component::{ActionRowComponent, InputTextStyle},
        interaction::{
            application_command::ApplicationCommandInteraction,
            message_component::MessageComponentInteraction, InteractionResponseType,
        },
    },
    prelude::Context,
};

use models::{error, job, SpeedParameters, Video};
use tokio_stream::StreamExt;

use crate::{
    commands::edit::EditMessage,
    utils::{self, durationparser::DisplayTimestamp},
};

pub async fn get_info(
    cmd: &ApplicationCommandInteraction,
    interaction_reponse: &MessageComponentInteraction,
    ctx: &Context,
    video: &Video,
) -> Result<job::Parameters, error::Interaction> {
    // Get media streams
    let mut msg_stream = crate::commands::edit::get_streams(&video).await?;
    cmd.edit(&ctx.http, &format!("Analyse de **{}**...", video.filename))
        .await?;
    // Wait for reponse
    let micros = loop {
        let payload: String = msg_stream
            .next()
            .await
            .ok_or(error::Interaction::Error)?
            .get_payload()?;
        let progress: job::Progress = serde_json::from_str(&payload.as_str())?;
        match progress {
            job::Progress::Started => {}
            job::Progress::Error(err) => {
                println!("Erreur du worker: {:?}", err);
                return Err(error::Interaction::Error);
            }
            job::Progress::Response(res) => match res {
                job::Response::GetStreams(res) => {
                    cmd.edit(
                        &ctx.http,
                        &format!(
                            "Attente de la réponse de l'utilisateur pour **{}**...",
                            video.filename
                        ),
                    )
                    .await?;
                    let stream = res.first().ok_or(error::Interaction::Error)?;
                    break stream.duration;
                }
            },
            job::Progress::Progress(_) => todo!(),
            job::Progress::Done(_) => todo!(),
        }
    };

    // Format micros as video timestamp
    let video_duration = std::time::Duration::from_micros(micros as u64);
    let duration = chrono::Duration::from_std(video_duration)?;

    let display_timestamp = duration.display_timestamp()?;

    // Display modal asking for target size
    interaction_reponse
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::Modal)
                .interaction_response_data(|modal| {
                    modal
                        .content("Vitesse")
                        .custom_id("speed_text")
                        .title(format!("Nouvelle vitesse ou durée du média"))
                        .components(|comp| {
                            comp.create_action_row(|row| {
                                row.create_input_text(|menu| {
                                    menu.custom_id("speed_text");
                                    menu.placeholder(format!(
                                        "Durée actuelle: {}",
                                        display_timestamp
                                    ));
                                    menu.style(InputTextStyle::Short);
                                    menu.label("Nouvelle durée");
                                    menu.max_length(10);
                                    menu.required(false)
                                })
                            });
                            comp.create_action_row(|row| {
                                row.create_input_text(|menu| {
                                    menu.custom_id("speed_factor_text");
                                    menu.placeholder("Ex: 1,5 - 2 - 0.5");
                                    menu.style(InputTextStyle::Short);
                                    menu.label("Ou Vitesse");
                                    menu.max_length(10);
                                    menu.required(false)
                                })
                            })
                        })
                })
        })
        .await?;

    // Get message of interaction reponse
    let interaction_reponse = &interaction_reponse
        .get_interaction_response(&ctx.http)
        .await?;

    // Await modal reponse
    let interaction = match interaction_reponse
        .await_modal_interaction(&ctx)
        .timeout(Duration::from_secs(60 * 3))
        .await
    {
        Some(x) => x,
        None => {
            cmd.edit(&ctx.http, "T trop lent, j'ai pas ton temps")
                .await?;
            return Err(error::Interaction::Timeout);
        }
    };
    // Extract target timestamp
    let end: &ActionRowComponent = &interaction.data.components[0].components[0];
    let end = match end {
        ActionRowComponent::InputText(txt) => &txt.value,
        _ => return Err(error::Interaction::Error),
    };
    let parsed = utils::durationparser::parse(end)?;
    let speed_factor = if parsed.is_zero() {
        match &interaction.data.components[1].components[0] {
            ActionRowComponent::InputText(txt) => &txt.value,
            _ => return Err(error::Interaction::Error),
        }.replace(",", ".").parse()?
    } else {
        duration
            .num_microseconds()
            .ok_or(error::Interaction::Error)? as f64
            / parsed.num_microseconds().ok_or(error::Interaction::Error)? as f64
    };

    // Ack modal interaction
    interaction.defer(&ctx.http).await?;
    Ok(job::Parameters::Speed(SpeedParameters { speed_factor }))
}
