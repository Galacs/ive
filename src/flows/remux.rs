use std::{num::ParseFloatError, time::Duration};

use serenity::{
    model::prelude::{
        component::{ActionRowComponent, InputTextStyle},
        interaction::{
            application_command::ApplicationCommandInteraction,
            message_component::MessageComponentInteraction, InteractionResponseType,
        },
        Message,
    },
    prelude::Context,
};

use models::{
    CutParameters, EncodeParameters, EncodeToSizeParameters, InteractionError, InvalidInputError,
    RemuxParameters, VideoContainer,
};

use crate::commands::edit::EditMessage;

pub async fn get_info(
    cmd: &ApplicationCommandInteraction,
    ctx: &Context,
    original_msg: &Message,
) -> Result<EncodeParameters, InteractionError> {
    // Display modal asking for target size
    // Create interaction response asking what edit to apply
    let message = cmd.create_followup_message(&ctx.http, |m| {
        m.content(format!(
            "Vers quel format convertir **{}** ?",
            original_msg.attachments[0].filename
        ));
        m.components(|comps| {
            comps.create_action_row(|row| {
                row.create_select_menu(|menu| {
                    menu.custom_id("container");
                    menu.placeholder("Choisissez un format");
                    menu.options(|f| {
                        f.create_option(|o| o.label("mp4").value("mp4"));
                        f.create_option(|o| o.label("mkv").value("mkv"))
                    })
                })
            })
        })
    })
    .await?;

    // Delete original interaction message
    cmd.delete_original_interaction_response(&ctx.http).await?;

    // Await edit apply choice (with timeout)
    let Some(interaction) = cmd.get_followup_message(&ctx.http, message).await?
        .await_component_interaction(&ctx)
        .timeout(Duration::from_secs(60 * 3))
        .await else {
        cmd.edit(&ctx.http, "T trop lent, j'ai pas ton temps").await?;
        return Err(InteractionError::Timeout);
    };

    // Ack modal interaction
    // interaction.defer(&ctx.http).await?;

    // Get edit kind from awaited interaction

    let container = match interaction.data.values[0].as_str() {
        "mp4" => VideoContainer::MP4,
        "mkv" => VideoContainer::MKV,
        _ => {
            return Err(models::InteractionError::InvalidInput(
                InvalidInputError::Error,
            ))
        }
    };

    Ok(EncodeParameters::Remux(RemuxParameters { container }))
}
