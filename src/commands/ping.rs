use serenity::builder::CreateApplicationCommand;
use serenity::model::prelude::component::InputTextStyle;
use serenity::model::prelude::interaction::application_command::{
    ApplicationCommandInteraction, CommandDataOption,
};
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
    }
    Ok(())
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("ping").description("ping")
}
