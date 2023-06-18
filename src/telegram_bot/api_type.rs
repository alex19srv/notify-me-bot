/*
 * Copyright 2023 Alex Syrnikov <alex.syrnikov19@gmail.com>
 * SPDX-License-Identifier: Apache-2.0
 */

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize,Debug)]
pub struct QueryResult {
    pub ok: bool,
    pub result:      Option<serde_json::Value>, // result on success
    pub description: Option<String>, // human-readable description of the result
    // pub parameters: Option<ResponseParameters>,
}

#[derive(Serialize, Deserialize,Debug)]
pub struct ApiUser { // https://core.telegram.org/bots/api#user
    pub id: i64,
    pub is_bot: bool,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub language_code: Option<String>,
    pub is_premium: Option<bool>,
    pub added_to_attachment_menu: Option<bool>,
    pub can_join_groups: Option<bool>,
    pub can_read_all_group_messages: Option<bool>,
    pub supports_inline_queries: Option<bool>,
}
#[derive(Serialize,Debug,Default)]
pub struct GetUpdatesParams { // https://core.telegram.org/bots/api#getupdates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_updates: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize,Debug,Default)]
pub struct ApiUpdate { // https://core.telegram.org/bots/api#update
    pub update_id: i64,
    pub message: Option<ApiMessage>,
    pub edited_message: Option<ApiMessage>,
}

#[derive(Serialize,Debug)]
pub struct SendMessageParams<'a> { // https://core.telegram.org/bots/api#sendmessage
    pub chat_id: i64,
    pub text: &'a str,
}

#[derive(Serialize, Deserialize,Debug,Default)]
pub struct ApiMessage { // https://core.telegram.org/bots/api#message
    pub message_id: i64,
    #[serde(rename = "from")]
    pub from_user: Option<ApiUser>,
    pub date: i64, // Date the message was sent in Unix time
    pub chat: ApiChat, // Conversation the message belongs to
    pub text: Option<String>, // For text messages, the actual UTF-8 text of the message
    pub entities: Option<Vec<ApiMessageEntity>>, // For text messages, special entities like usernames,
}

#[derive(Serialize, Deserialize,Debug,Default)]
pub struct ApiChat { // https://core.telegram.org/bots/api#chat
    pub id: i64,
}
#[derive(Serialize, Deserialize,Debug,Default)]
pub struct ApiMessageEntity {}

#[derive(Serialize,Debug,Default)]
pub struct SetMyCommandsParams<'a> { // https://core.telegram.org/bots/api#setmycommands
    pub commands: Vec<ApiBotCommand<'a>>,
}

#[derive(Serialize,Debug)]
pub struct ApiBotCommand<'a> { // https://core.telegram.org/bots/api#botcommand
    pub command: &'a str,
    pub description: &'a str,
}

#[derive(Serialize,Debug,Default)]
pub struct SetWebhookParams<'a> { // https://core.telegram.org/bots/api#setwebhook
    pub url: &'a str,
    pub secret_token: &'a str,
}
