use std::path::Path;
use std::time::Duration;

use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::component::{ActionRowComponent, InputText, InputTextStyle};
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption,
};
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::prelude::Context;

use tokio::fs::{self, File, remove_dir_all};
use tokio::io::AsyncWriteExt;

pub async fn run(cmd: &ApplicationCommandInteraction, ctx: &Context) -> Result<(), String> {
    let message_id = &cmd.data.target_id.unwrap().to_message_id();
    let message = &cmd.data.resolved.messages.get(message_id).unwrap();
    if message.attachments.len() != 1 {
        if let Ok(_) = cmd
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.content("Le message doit contenir exlusivement une seule vidéo")
                    })
            })
            .await
        {
            return Ok(());
        }
    }

    if let Err(why) = cmd
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|m| {
                    m.content(format!(
                        "Que voulez vous faire avec **{}**...",
                        message.attachments[0].filename
                    ));
                    m.components(|comps| {
                        comps.create_action_row(|row| {
                            row.create_select_menu(|menu| {
                                menu.custom_id("edit_type");
                                menu.placeholder("Choisissez une modification");
                                menu.options(|f| {
                                    f.create_option(|o| {
                                        o.label("Changer la taille du fichier")
                                            .value("encode_to_size")
                                    })
                                })
                            })
                        })
                    })
                })
        })
        .await
    {
        println!("Cannot respond to slash command: {}", why);
    }

    let prompt_msg = &cmd.get_interaction_response(&ctx.http).await.unwrap();

    let interaction = match prompt_msg
        .await_component_interaction(&ctx)
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
            return Ok(());
        }
    };

    let edit_type = &interaction.data.values[0];

    let mut t_size = 0.0;

    match edit_type.as_str() {
        "encode_to_size" => {
            if let Err(why) = interaction
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
                                                message.attachments[0].size as f64
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
            let prompt_msg = &interaction
                .get_interaction_response(&ctx.http)
                .await
                .unwrap();

            let interaction = match prompt_msg
                .await_modal_interaction(&ctx)
                .timeout(Duration::from_secs(60 * 3))
                .await
            {
                Some(x) => x,
                None => {
                    // cmd.edit_original_interaction_response(&ctx.http, |response| {
                    //     response
                    //         .content("T trop lent, j'ai pas ton temps")
                    //         .components(|comp| comp)
                    // })
                    // .await
                    // .unwrap();
                    return Ok(());
                }
            };
            let input: &ActionRowComponent = &interaction.data.components[0].components[0];
            t_size = match input {
                ActionRowComponent::InputText(txt) => txt.value.parse::<f32>().unwrap(),
                _ => 0.0,
            };
            if let Err(why) = interaction.defer(&ctx.http).await {
                println!("Cannot respond to slash command: {}", why);
            }
        }
        _ => {}
    }

    if let Err(why) = cmd
        .edit_original_interaction_response(&ctx, |r| {
            r.content(format!(
                "Telechargement de **{}**...",
                message.attachments[0].filename
            ))
            .components(|comp| comp)
        })
        .await
    {
        println!("Cannot respond to slash command: {}", why);
    }
    let file = message.attachments[0].download().await.unwrap();

    let dir = Path::new("tmpfs").join(format!("{}", cmd.token));
    let dir = std::env::current_dir().unwrap().join(dir);

    if let Err(why) = fs::create_dir(&dir).await {
        println!("Error creating directory: {}", why)
    }

    let path = dir.join(&message.attachments[0].filename);

    let mut buffer = File::create(&path).await.unwrap();

    if let Err(why) = buffer.write_all(&file).await {
        println!("Error saving file: {}", why)
    };

    if let Err(why) = interaction
        .edit_original_interaction_response(&ctx.http, |response| {
            response
                .content(format!(
                    "Modification de **{}**...",
                    message.attachments[0].filename
                ))
                .components(|comp| comp)
        })
        .await
    {
        println!("Cannot respond to slash command: {}", why);
    }

    let dest_file = dir.join(&format!("edit-{}", message.attachments[0].filename));

    if let Err(why) = ffedit::encoding::encode_to_size(&path, t_size, dest_file.to_str().unwrap())
    {
        println!("Error encoding file: {:?}", why);
    }

    if let Err(why) = interaction
        .edit_original_interaction_response(&ctx.http, |response| {
            response
                .content(format!(
                    "Envoi de **{}** modifié...",
                    message.attachments[0].filename
                ))
                .components(|comp| comp)
        })
        .await
    {
        println!("Cannot respond to slash command: {}", why);
    }

    let paths = vec![dest_file.to_str().unwrap()];

    if let Err(why) = cmd
        .channel_id
        .send_files(&ctx.http, paths, |m| {
            m.content(format!("**{}**:", message.attachments[0].filename))
        })
        .await
    {
        println!("Error uploading file: {}", why);
    }

    if let Err(why) = remove_dir_all(dir).await {
        println!("Can't delete videos directory: {}", why);
    }

    if let Err(why) = interaction
        .edit_original_interaction_response(&ctx.http, |response| {
            response
                .content(format!(
                    "**{}** à été modifié avec success",
                    message.attachments[0].filename
                ))
                .components(|comp| comp)
        })
        .await
    {
        println!("Cannot respond to slash command: {}", why);
    }

    Ok(())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("Edit video")
}
