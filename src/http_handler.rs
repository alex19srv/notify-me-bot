/*
 * Copyright 2023 Alex Syrnikov <alex.syrnikov19@gmail.com>
 * SPDX-License-Identifier: Apache-2.0
 */

use std::{
    sync::Arc
    ,time::SystemTime
};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use axum::{
    response::IntoResponse
    ,extract::State
    ,http::{StatusCode,HeaderMap}
    ,Json
};
use base64::Engine;
use crate::{db, state::AppState, telegram_bot};
use telegram_bot::{api_type, TelegramBot};
// use serde_json::Value;

pub async fn handle_webhook(
    State(state): State<Arc<Mutex<AppState>>>,
    headers: HeaderMap
    // ,Json(payload): Json<Value>
    ,Json(api_update): Json<api_type::ApiUpdate>
) -> impl IntoResponse {
    // FIXME: use Json(payload): Json<Value> and convert to api_updates by hand to catch
    // conversion errors
    let webhook_token = match headers.get("X-Telegram-Bot-Api-Secret-Token") {
        None => "",
        Some(token) => match token.to_str() {
            Ok(token) => token,
            Err(_) => ""
        },
    };
    println!("got update {api_update:?}");
    {
        let current_time = SystemTime::now();
        let mut state_guard = state.lock().unwrap();
        if let Ok(n) = current_time.duration_since(state_guard.prev_query_time) {
            state_guard.prev_query_time = current_time;
            if n.as_secs() < 5 {
                println!("delay {}, will switch to polling", n.as_secs());
                tokio::spawn( async move {
                    TelegramBot::set_mode_polling().await.unwrap();
                });
            }
        }
    }

    if let Err(err) = TelegramBot::handle_webhook_update( webhook_token, api_update ).await {
        println!("handle_webhook got error in TelegramBot::handle_webhook_update: {err:?}");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    StatusCode::OK
}

pub async fn handle_message(
    Json(message_request): Json<SendMessageRequest>,
) -> (StatusCode, Json<QueryResult>) {
    // db: find user id by token
    let mut token:[u8;40] = [0;40]; // token size 32 butes, but base64 decode estimates not perfect
    let token_size = match base64::engine::general_purpose::STANDARD_NO_PAD
        .decode_slice(&message_request.token,&mut token)
    {
        Err(err) => {
            tracing::error!("Failed decode token string with error {err:?},\n token: \"{}\"", message_request.token);
            return (StatusCode::BAD_REQUEST
                    ,Json(QueryResult::error("BAD_REQUEST".to_string(),Some("Failed base64 decode token".to_string()))));
        },
        Ok(len) => len,
    };
    println!("decoded token size {token_size}");
    // db::add_session(&token[..token_size], 19).await;
    let chat_id = match db::find_chat_by_token(&token[..token_size]).await {
        Err(err) => {
            tracing::error!("Failed find user by token {err:?},\n token: \"{}\"", message_request.token);
            return (StatusCode::INTERNAL_SERVER_ERROR
                    ,Json(QueryResult::error("SERVER_ERROR".to_string(),None)));
        },
        Ok(opt) => opt,
    };
    let chat_id = match chat_id {
        None => {
            return (StatusCode::UNAUTHORIZED
                    ,Json(QueryResult::error("UNAUTHORIZED".to_string(),Some("token not found".to_string()))));
        },
        Some(chat_id) => chat_id,
    };
    println!("Found user id {chat_id} for token {}", &message_request.token);

    // FIXME: telegram: send message to user
    TelegramBot::send_message(chat_id, &message_request.message ).await;

    (StatusCode::OK, Json(QueryResult::ok()))
}
pub async fn handle_options(
    // Json(message_request): Json<SendMessageRequest>,

) -> (StatusCode, Json<QueryResult>) {
    (StatusCode::OK, Json(QueryResult::ok()))
}

#[derive(Deserialize,Debug)]
pub struct SendMessageRequest {
    pub token: String,
    pub message: String,
}

#[derive(Serialize,Default)]
pub struct QueryResult {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}
impl QueryResult {
    fn ok() -> QueryResult {
        let mut result = QueryResult::default();
        result.status = "OK".to_string();
        return result;
    }
    fn error(status: String, message: Option<String>) -> QueryResult {
        QueryResult {status, message}
    }
}
