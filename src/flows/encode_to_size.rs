use std::time::Duration;

use serenity::{
    model::prelude::{
        component::{ActionRowComponent, InputTextStyle},
        interaction::{message_component::MessageComponentInteraction, InteractionResponseType, application_command::ApplicationCommandInteraction},
    },
    prelude::Context,
};

use models::{job, EncodeToSizeParameters, error};

use crate::commands::edit::{EditMessage, GetMessage};

pub async fn get_info(
    cmd: &ApplicationCommandInteraction,
    interaction_reponse: &MessageComponentInteraction,
    ctx: &Context
) -> Result<job::Parameters, error::Interaction> {
    let message = cmd.get_message()?;
    // Display modal asking for target size
    interaction_reponse.create_interaction_response(&ctx.http, |response| {
        response
            .kind(InteractionResponseType::Modal)
            .interaction_response_data(|modal| {
                modal
                    .content("Taille")
                    .custom_id("size_text")
                    .title(format!("Quelle taille doit faire le fichier ?"))
                    .components(|comp| {
                        comp.create_action_row(|row| {
                            row.create_input_text(|menu| {
                                menu.custom_id("size_text");
                                menu.placeholder(format!(
                                    "Taille actuelle: {:.2} Mo",
                                    message.attachments[0].size as f64 / 2_i32.pow(20) as f64
                                ));
                                menu.style(InputTextStyle::Short);
                                menu.label("Taille");
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
            return Err(error::Interaction::Timeout);
        }
    };
    // Extract target size from modal response
    let input: &ActionRowComponent = &interaction.data.components[0].components[0];
    let t_size = match input {
        ActionRowComponent::InputText(txt) => txt.value.parse::<f32>(),
        _ => Ok(0.0),
    };

    // Ack modal interaction
    interaction.defer(&ctx.http).await?;

    match t_size {
        Err(err) => {
            cmd.edit(&ctx.http, "Veuillez donner un nombre").await?;
            Err(error::Interaction::InvalidInput(error::InvalidInput::StringParse(err)))
        },
        Ok(t) => {
            if t > 45.0 {
                cmd.edit(&ctx.http, &format!("**{}** ne pourra pas être envoyé car {}Mo > 25Mo (limite de discord)", message.attachments[0].filename, t)).await?;
                Err(error::Interaction::InvalidInput(error::InvalidInput::Error))
            } else {
                Ok(job::Parameters::EncodeToSize(EncodeToSizeParameters {
                    target_size: (t * 2_f32.powf(20.0)) as u32,
                }))
            }
        }
    }
}
