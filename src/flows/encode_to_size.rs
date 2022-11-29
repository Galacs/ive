use std::time::Duration;

use serenity::{
    model::prelude::{
        component::{ActionRowComponent, InputTextStyle},
        interaction::{message_component::MessageComponentInteraction, InteractionResponseType},
        Message,
    },
    prelude::Context,
};

use models::{EncodeParameters, EncodeToSizeParameters, InteractionError};

pub async fn get_info(
    cmd: &MessageComponentInteraction,
    ctx: &Context,
    original_msg: &Message,
) -> Result<EncodeParameters, InteractionError> {
    // Display modal asking for target size
    cmd.create_interaction_response(&ctx.http, |response| {
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
                                    original_msg.attachments[0].size as f64 / 2_i32.pow(20) as f64
                                ));
                                menu.style(InputTextStyle::Short);
                                menu.label("Taille")
                            })
                        })
                    })
            })
    })
    .await?;
    // Get message of interaction reponse
    let interaction_reponse = &cmd.get_interaction_response(&ctx.http).await?;

    // Await modal reponse
    let interaction = match interaction_reponse
        .await_modal_interaction(&ctx)
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
    // Extract target size from modal response
    let input: &ActionRowComponent = &interaction.data.components[0].components[0];
    let t_size = match input {
        ActionRowComponent::InputText(txt) => txt.value.parse::<f32>()?,
        _ => 0.0,
    };
    // Ack modal interaction
    interaction.defer(&ctx.http).await?;

    // Return target size
    Ok(EncodeParameters::EncodeToSize(EncodeToSizeParameters {
        target_size: (t_size * 2_f32.powf(20.0)) as u32,
    }))
}
