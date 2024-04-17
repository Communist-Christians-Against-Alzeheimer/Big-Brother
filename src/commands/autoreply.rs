use std::collections::VecDeque;

use crate::{context::Context, structs::Rule};
use sqlx::query;
use twilight_model::{
    application::interaction::application_command::{
        CommandData, CommandDataOption, CommandOptionValue,
    },
    channel::message::MessageFlags,
    http::interaction::InteractionResponseData,
    id::{marker::GuildMarker, Id},
};
use twilight_util::builder::InteractionResponseDataBuilder;

async fn add(
    trigger: String,
    reply: String,
    guild: Id<GuildMarker>,
    ctx: &Context,
) -> anyhow::Result<String> {
    if ctx
        .data
        .read()
        .await
        .rules
        .iter()
        .any(|r| r.trigger == trigger)
    {
        return Ok("Rule already exists, delete it if you want to replace it.".to_owned());
    };

    query!(
        "insert into rules values ($1, $2, $3)",
        trigger,
        reply,
        guild.to_string()
    )
    .execute(&ctx.data.read().await.db)
    .await?;

    let out = format!("Added rule `{}`!", trigger);

    ctx.data.write().await.rules.push(Rule {
        trigger,
        reply,
        guild,
    });

    Ok(out)
}
async fn remove(trigger: String, ctx: &Context) -> anyhow::Result<String> {
    if !ctx
        .data
        .read()
        .await
        .rules
        .iter()
        .any(|r| r.trigger == trigger)
    {
        return Ok("The rule you're trying to remove doesn't exist.".to_owned());
    };

    query!("delete from rules where trigger = $1", trigger)
        .execute(&ctx.data.read().await.db)
        .await?
        .rows_affected();

    let mut data = ctx.data.write().await;
    data.rules = data
        .rules
        .iter()
        .filter_map(|r| {
            if r.trigger != trigger {
                Some(r.clone())
            } else {
                None
            }
        })
        .collect();

    Ok(format!("Removed rule `{trigger}`"))
}
async fn list(guild: Id<GuildMarker>, ctx: &Context) -> anyhow::Result<String> {
    let mut out = String::new();

    let rules = ctx
        .data
        .read()
        .await
        .rules
        .iter()
        .filter(|r| r.guild == guild);

    Ok(out)
}
pub async fn interaction(
    cmd: &CommandData,
    ctx: &Context,
) -> anyhow::Result<InteractionResponseData> {
    let get_str = |o: CommandDataOption| -> String {
        if let CommandOptionValue::String(s) = o.value {
            s
        } else {
            unreachable!()
        }
    };
    // This is a subcommand, it'll always be here
    let subcommand = &cmd.options[0];
    let mut args: VecDeque<_> = match &subcommand.value {
        CommandOptionValue::SubCommand(c) => c.clone(),
        _ => unreachable!(),
    }
    .into();
    let (trigger, reply) = (args.pop_front().map(get_str), args.pop_front().map(get_str));

    Ok(InteractionResponseDataBuilder::new()
        .content(match subcommand.name.as_str() {
            "add" => {
                add(
                    trigger.unwrap(),
                    reply.unwrap(),
                    cmd.guild_id.expect("this is only going to run in a guild"),
                    ctx,
                )
                .await
            }
            "remove" => remove(trigger.unwrap(), ctx).await,
            "list" => {
                list(
                    cmd.guild_id.expect("this is only going to run in a guild"),
                    ctx,
                )
                .await
            }
            _ => unreachable!(),
        }?)
        .flags(MessageFlags::EPHEMERAL)
        .build())
}
