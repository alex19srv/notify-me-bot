/*
 * Copyright 2023 Alex Syrnikov <alex.syrnikov19@gmail.com>
 * SPDX-License-Identifier: Apache-2.0
 */

use std::collections::HashMap;
use tokio::time::{sleep, Duration};
use base64::Engine;
use crate::error::BotError;
use crate::{db, random};
use crate::telegram_bot::{ api_type, TelegramBot };

pub async fn poll_updates() -> Result<(), BotError> {
    println!("Telegram poller started");
    let mut next_update_id = 0;
    let mut empty_poll_count = 0;
    loop {
        let updates = match TelegramBot::get_updates(next_update_id).await {
            Ok(updates) => { updates },
            Err(err) => {
                tracing::error!("TelegramBot::get_updates() got error {err:?}");
                sleep(Duration::from_millis(1000)).await;
                continue;
            },
        };
        if updates.len() == 0 {
            empty_poll_count += 1;
            if empty_poll_count >= 3 {
                break;
            }
        } else {
            empty_poll_count = 0;
            next_update_id = handle_updates(updates).await?;
        }
    }
    TelegramBot::set_mode_webhook().await?;

    return Ok(());
}
pub async fn handle_updates(updates: Vec<api_type::ApiUpdate>) -> Result<i64, BotError> {
    let mut next_update_id = 0;

    for update in updates {
        let update_id = update.update_id;
        if update_id >= next_update_id {
            next_update_id = update_id + 1;
        }
        handle_update(update).await?;
    }

    return Ok(next_update_id);
}

pub async fn handle_update( update: ApiUpdate ) -> Result<(),BotError> {
    let message = match update.message {
        None => return Ok(()),
        Some(message) => message
    };
    let chat_id = message.chat.id;
    let text = match message.text {
        None => return Ok(()),
        Some(text) => text
    };
    if let Err(err) = handle_message(chat_id, &text).await {
        tracing::error!("handle_message() got error processing chat {chat_id},with text\n'{text}'\n{err:?}");
    }

    Ok(())
}

async fn handle_message(chat_id: i64, text: &str) -> Result<(),BotError> {
    println!("processing text from chat_id {chat_id} \"{text}\"");
    let text = text.trim();
    if text.starts_with("/") {
        match extract_command(text) {
            None => return Ok(()),
            Some((command,tail)) => {
                handle_command(chat_id, command, tail).await?;
                return Ok(())
            },
        };
    }

    return Ok(());
}
fn extract_command(text: &str) -> Option<(Command,&str)>{
    let text_pair: Vec<&str> = text.splitn(2, &[' ', '\t']).collect();
    // let r: Vec<&str> = text.splitn(2, |c| {c==' '||c=='\t'}).collect();
    if text_pair.len() < 1 {
        return None;
    }
    let cmd_text = text_pair[0];
    let command = match cmd_map().get(cmd_text) {
        None => return None,
        Some(cmd) => cmd.command.clone(),
    };
    if text_pair.len() >= 2 {
        return Some((command,text_pair[1].trim()));
    }

    return Some((command,""));
}
async fn handle_command(chat_id: i64, command: Command, tail: &str) -> Result<(),BotError> {
    match command {
        Command::Start => handle_start(chat_id, tail).await?,
        Command::Stop => handle_stop(chat_id).await?,
        Command::Help => handle_help(chat_id).await?,
        Command::ShowToken => handle_show_token(chat_id).await?,
        Command::UpdateToken => handle_update_token(chat_id).await?,
    }

    return Ok(());
}

async fn handle_start(chat_id: i64, tail: &str) -> Result<(),BotError> {
    println!("/start handler for chat {chat_id} with tail \"{tail}\"");
    match db::find_token_by_chat(chat_id).await? {
        None => {
            let mut token: [u8; 32] = [0; 32];
            random::gen_random(&mut token[..])?;
            db::create_session(&token, chat_id).await?;
            let token_str = base64::engine::general_purpose::STANDARD_NO_PAD.encode(&token);
            let response_message = format!(
                "generated token\n\n\
                {token_str}\n\n\
                use it to send requests using notify-me-api npm package"
            );
            TelegramBot::send_message(chat_id, &response_message).await?;
        },
        Some(token) => {
            let token_str = base64::engine::general_purpose::STANDARD_NO_PAD.encode(&token);
            let response_message = format!(
                "token already exist for your chat\n\n\
                {token_str}\n\n\
                use it to send requests using notify-me-api npm package"
            );
            TelegramBot::send_message(chat_id, &response_message).await?;
        }
    }
    Ok(())
}
async fn handle_stop( chat_id: i64 ) -> Result<(),BotError> {
    println!("/stop handler for chat {chat_id}");
    db::delete_session(chat_id).await?;

    let response_message = format!(
        "Deleted token for this chat.\n\n\
        You will not receive any messages from this bot until next /start."
    );
    TelegramBot::send_message(chat_id, &response_message).await?;
    Ok(())
}
async fn handle_help( chat_id: i64 ) -> Result<(),BotError> {
    println!("/help handler for chat {chat_id}");

    let mut cmd_list_message = String::new();
    for cmd in CMD_LIST {
        cmd_list_message.push_str(&format!("{} {}\n", cmd.name, cmd.description));
    }
    let response_message = format!(
        "This bot allow to send messages from web to telegram chats.\n\
        First connect to this bot (/start) and get token. \
        Use it to send requests using notify-me-api npm package\n\n\
        Available commands:\n{cmd_list_message}"
    );
    TelegramBot::send_message(chat_id, &response_message).await?;
    Ok(())
}
async fn handle_show_token( chat_id: i64 ) -> Result<(),BotError> {
    println!("/show_token handler for chat {chat_id}");

    match db::find_token_by_chat(chat_id).await? {
        None => {
            let response_message = format!(
                "Token not found, this chat not connected to bot.\n\n\
                run /start to connect and get token."
            );
            TelegramBot::send_message(chat_id, &response_message).await?;
        }
        Some(token) => {
            let token_str = base64::engine::general_purpose::STANDARD_NO_PAD.encode(&token);
            let response_message = format!(
                "your token:\n\n\
                {token_str}\n\n\
                use it to send requests using notify-me-api npm package"
            );
            TelegramBot::send_message(chat_id, &response_message).await?;
        }
    }

    Ok(())
}
async fn handle_update_token( chat_id: i64 ) -> Result<(),BotError>
{
    println!("/start handler for chat {chat_id}");
    let mut token: [u8; 32] = [0; 32];
    random::gen_random(&mut token[..])?;
    db::create_session(&token, chat_id).await?;
    let token_str = base64::engine::general_purpose::STANDARD_NO_PAD.encode(&token);
    let response_message = format!(
        "new token\n\n\
        {token_str}\n\n\
        use it to send requests using notify-me-api npm package"
    );
    TelegramBot::send_message(chat_id, &response_message).await?;

    return Ok(());
}

#[derive(Debug,Clone)]
enum Command {
    Start,
    Stop,
    Help,
    ShowToken,
    UpdateToken,
}
#[derive(Debug)]
pub struct TelegramCommand {
    pub name: &'static str,
    command: Command,
    pub description: &'static str,
}
pub static CMD_LIST: &[TelegramCommand] = &[ // FIXME: combine this list and enum Command
    TelegramCommand{ name: "/start", command: Command::Start
        , description: "Start chat (connect to bot)" },
    TelegramCommand{ name: "/stop", command: Command::Stop
        , description: "Stop chat (disconnect from bot)"},
    TelegramCommand{ name: "/help", command: Command::Help, description: "Show commands"},
    TelegramCommand{ name: "/show_token", command: Command::ShowToken
        , description: "Show my current token"},
    TelegramCommand{ name: "/update_token", command: Command::UpdateToken
        , description: "Update current token"},
];

use once_cell::sync::OnceCell;
use crate::telegram_bot::api_type::ApiUpdate;

static CMD_MAP: OnceCell<HashMap<&str,&TelegramCommand>> = OnceCell::new();
#[inline]
fn cmd_map() -> &'static HashMap<&'static str,&'static TelegramCommand>
{
    unsafe { CMD_MAP.get_unchecked() }
}

pub fn init() -> Result<(), BotError>
{
    let mut cmd_map: HashMap<&str,&TelegramCommand> = HashMap::new();
    for cmd in CMD_LIST {
        cmd_map.insert(cmd.name, cmd);
    }
    CMD_MAP.set(cmd_map).unwrap();

    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_command() {
        init();
        extract_command("");
        extract_command("/");
        extract_command("/ ");
        extract_command("/  ");
        extract_command("/\t 1");
        assert_eq!(2+2, 4);
    }
}
