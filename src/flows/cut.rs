use std::{time::Duration, num::ParseFloatError};

use serenity::{
    model::prelude::{
        component::{ActionRowComponent, InputTextStyle},
        interaction::{message_component::MessageComponentInteraction, InteractionResponseType, application_command::ApplicationCommandInteraction},
        Message,
    },
    prelude::Context,
};

use models::{EncodeParameters, EncodeToSizeParameters, InteractionError, CutParameters};

use crate::commands::edit::EditMessage;

pub async fn get_info(
    cmd: &ApplicationCommandInteraction,
    interaction_reponse: &MessageComponentInteraction,
    ctx: &Context
) -> Result<EncodeParameters, InteractionError> {
    // Display modal asking for target size
    interaction_reponse.create_interaction_response(&ctx.http, |response| {
        response
            .kind(InteractionResponseType::Modal)
            .interaction_response_data(|modal| {
                modal
                    .content("Durée de la vidéo")
                    .custom_id("size_text")
                    .title(format!("Quelle taille doit faire le fichier ?"))
                    .components(|comp| {
                        comp.create_action_row(|row| {
                            row.create_input_text(|menu| {
                                menu.custom_id("start");
                                menu.placeholder("en secondes");
                                menu.style(InputTextStyle::Short);
                                menu.label("Début");
                                menu.max_length(10)
                            })
                        });
                        comp.create_action_row(|r| {
                            r.create_input_text(|menu| {
                                menu.custom_id("end");
                                menu.placeholder("en secondes");
                                menu.style(InputTextStyle::Short);
                                menu.label("Fin");
                                menu.max_length(10)
                            })
                        })
                    })
            })
    })
    .await?;
    // Get message of interaction reponse
    let interaction_reponse = &interaction_reponse.get_interaction_response(&ctx.http).await?;

    // Await modal reponse
    let interaction = match interaction_reponse
        .await_modal_interaction(&ctx)
        .timeout(Duration::from_secs(60 * 3))
        .await
    {
        Some(x) => x,
        None => {
            cmd.edit(&ctx.http, "T trop lent, j'ai pas ton temps").await?;
            return Err(InteractionError::Timeout);
        }
    };
    // Extract target size from modal response
    let start: &ActionRowComponent = &interaction.data.components[0].components[0];
    let start = match start {
        ActionRowComponent::InputText(txt) => txt.value.parse::<f32>()?,
        _ => return Err(InteractionError::Error),
    };
    let end: &ActionRowComponent = &interaction.data.components[1].components[0];
    let end = match end {
        ActionRowComponent::InputText(txt) => txt.value.parse::<f32>()?,
        _ => return Err(InteractionError::Error),
    };

    // Ack modal interaction
    interaction.defer(&ctx.http).await?;

    Ok(EncodeParameters::Cut(CutParameters {start: Some(start as u32), end: Some(end as u32) }))
}
