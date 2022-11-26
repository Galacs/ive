use std::{time::Duration, path::Path};

use serenity::{
    model::prelude::{
        component::{InputTextStyle, ActionRowComponent},
        interaction::{message_component::MessageComponentInteraction, InteractionResponseType},
        Message,
    },
    prelude::Context,
};

use models::{Job, EncodeToSizeParameters, Video, VideoURI};

pub async fn get_info(cmd: &MessageComponentInteraction, ctx: &Context, original_msg: &Message) -> Result<Job, ()>{
    // Display modal asking for target size
    if let Err(why) = cmd
        .create_interaction_response(&ctx.http, |response| {
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
                                        original_msg.attachments[0].size as f64
                                            / 2_i32.pow(20) as f64
                                    ));
                                    menu.style(InputTextStyle::Short);
                                    menu.label("Taille")
                                })
                            })
                        })
                })
        })
        .await
    {
        println!("Cannot respond to slash command: {}", why);
    }
    // Get message of interaction reponse
    let interaction_reponse = &cmd.get_interaction_response(&ctx.http).await.unwrap();

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
            .await
            .unwrap();
            return Err(());
        }
    };
    // Extract target size from modal response
    let input: &ActionRowComponent = &interaction.data.components[0].components[0];
    let t_size = match input {
        ActionRowComponent::InputText(txt) => txt.value.parse::<f32>().unwrap(),
        _ => 0.0,
    };
    // Ack modal interaction
    if let Err(why) = interaction.defer(&ctx.http).await {
        println!("Cannot respond to slash command: {}", why);
    };

    // Return target size
    Ok(Job::EncodeToSize(None, EncodeToSizeParameters { target_size: (t_size * 2_f32.powf(20.0)) as u32 }))
}

pub async fn run(path: &Path, dest_file: &str, params: EncodeToSizeParameters) {
    let video_uri = VideoURI::Path(path.to_str().unwrap().to_owned());
    let video = Video::new(video_uri, None);
    if let Err(why) = ffedit::encoding::encode_to_size(&video, params, dest_file) {
        println!("Error encoding file: {:?}", why);
    }
}