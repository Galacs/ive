use std::time::Duration;

use serenity::{
    model::prelude::{
        interaction::{
            application_command::ApplicationCommandInteraction,
            message_component::MessageComponentInteraction,
        },
    },
    prelude::Context,
};

use models::{
    job, error,
    RemuxParameters, VideoContainer,
};

use crate::commands::edit::{EditMessage, GetMessage};

pub async fn get_info(
    cmd: &ApplicationCommandInteraction,
    interaction_reponse: &MessageComponentInteraction,
    ctx: &Context
) -> Result<job::Parameters, error::Interaction> {
    // Create interaction response asking what edit to apply
    interaction_reponse.defer(&ctx.http).await?;

    let sender_message = cmd.get_message()?;

    cmd.edit_original_interaction_response(&ctx.http, |m| {
        m.content(format!(
            "Vers quel format convertir **{}** ?",
            sender_message.attachments[0].filename
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
    }).await?;

    // Await edit apply choice (with timeout)
    let interaction_reponse = &cmd.get_interaction_response(&ctx.http).await?;
    let Some(interaction) = interaction_reponse
        .await_component_interaction(&ctx)
        .timeout(Duration::from_secs(60 * 3))
        .await else {
        cmd.edit(&ctx.http, "T trop lent, j'ai pas ton temps").await?;
        return Err(error::Interaction::Timeout);
    };

    // Get edit kind from awaited interaction
    let container = match interaction.data.values[0].as_str() {
        "mp4" => VideoContainer::MP4,
        "mkv" => VideoContainer::MKV,
        _ => {
            return Err(error::Interaction::InvalidInput(
                error::InvalidInput::Error,
            ))
        }
    };

    Ok(job::Parameters::Remux(RemuxParameters { container }))
}
