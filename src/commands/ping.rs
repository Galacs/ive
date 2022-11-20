use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::prelude::Context;

pub async fn run(cmd: &ApplicationCommandInteraction, ctx: &Context) -> Result<(), String> {
    if let Err(why) = cmd
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| message.content("Pong !"))
        })
        .await
    {
        println!("Cannot respond to slash command: {}", why);
        return Err("Cannot respond to slash command".to_owned());
    }
    Ok(())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("ping").description("ping")
}
