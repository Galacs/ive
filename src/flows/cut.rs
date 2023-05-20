use std::time::Duration;

use serenity::{
    model::prelude::{
        component::{ActionRowComponent, InputTextStyle},
        interaction::{message_component::MessageComponentInteraction, InteractionResponseType, application_command::ApplicationCommandInteraction},
    },
    prelude::Context,
};

use models::{InteractionError, CutParameters, job};

use crate::commands::edit::EditMessage;

pub async fn get_info(
    cmd: &ApplicationCommandInteraction,
    interaction_reponse: &MessageComponentInteraction,
    ctx: &Context
) -> Result<job::Parameters, InteractionError> {
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

    match (start, end) {
        (s, e) if (s, e) < (0.0, 0.0) => cmd.edit(&ctx.http, "Les nombres ne peuvent pas être négatives").await?,
        (s, e) if s == 0.0 && e == 0.0 => cmd.edit(&ctx.http, "Les deux nombres de peuvent pas valoir 0").await?,
        (s, e) if s > e => cmd.edit(&ctx.http, "Le debut de la vidéo doit être avant la fin").await?,
        (s, e) => return Ok(job::Parameters::Cut(CutParameters {start: Some(s as u32), end: Some(e as u32) }))
    }
    Err(InteractionError::InvalidInput(models::InvalidInputError::Error))
}
