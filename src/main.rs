/*
 * Copyright 2023 Alex Syrnikov <alex.syrnikov19@gmail.com>
 * SPDX-License-Identifier: Apache-2.0
 */

use axum::{
    http::{self, header},
    response::IntoResponse,
    Router,
    routing::{get, post},
};
use tower_http::cors::{Any, CorsLayer};
use dotenv::dotenv;
use std::env;
use std::sync::{ Arc, Mutex };
use std::time::SystemTime;

mod error;
mod http_handler;
mod random;
pub mod telegram_bot;
mod db;
mod state;

use state::AppState;
use telegram_bot::TelegramBot;
use crate::error::BotError;

#[tokio::main]
async fn main() -> Result<(), BotError> {
    tracing_subscriber::fmt::init();

    dotenv().expect("Failed open .env file");
    let token   = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not found in .env file");
    let db_file = env::var("DB_FILE").expect("DB_FILE not found in .env file");
    let listen_addr = env::var("LISTEN_ADDR").expect("LISTEN_ADDR not found in .env file");
    let listen_port = env::var("LISTEN_PORT").expect("LISTEN_PORT not found in .env file");
    let webhook_url = env::var("TELEGRAM_WEBHOOK").expect("TELEGRAM_WEBHOOK not found in .env file");

    db::init(&db_file).await.expect("Failed init database");
    TelegramBot::init(&token, &webhook_url).await?;

    // telegram_bot.get_me().await;

    // telegram_bot.get_updates().await;

    let shared_state = Arc::new( Mutex::new(AppState { prev_query_time: SystemTime::now() }) );

    let app = Router::new()
        .route("/", get(root))
        .route("/scripts/notify-me.js", get(script_cjm))
        .route("/webhook", post(http_handler::handle_webhook))
        .route("/send-message", post(http_handler::handle_message))
        .route_layer(CorsLayer::new()
            .allow_methods([http::Method::GET, http::Method::POST, http::Method::OPTIONS])
            .allow_headers([http::header::CONTENT_TYPE])
            .allow_origin(Any))
        .with_state(shared_state);

    let bind_addr = format!("{listen_addr}:{listen_port}");
    axum::Server::bind(&bind_addr.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    return Ok(());
}

async fn root() -> &'static str {
    "Hello, World!"
}
pub async fn script_cjm() -> impl IntoResponse {
    let result = r#"
"use strict";
class NotifyMe {
    static async sendMessage(url, token, message) {
        const data = { token, message };
        const response = await fetch(url, {
            method: "POST",
            mode: "cors",
            cache: "no-cache",
            credentials: "omit",
            headers: {
                "Content-Type": "application/json",
            },
            redirect: "follow",
            referrerPolicy: "no-referrer",
            body: JSON.stringify(data),
        });
        const result = await response.json();
        if (!result?.status) {
            console.error("returned object have not valid format. Object:", result);
            throw new RangeError("returned object have not valid format");
        }
        const status = result.status;
        if (status === "OK") {
            return result;
        }
        if (result.message) {
            throw new Error(status + ": " + result.message);
        }
        else {
            throw new Error("" + status);
        }
    }
    static createSender(url, token) {
        return (message) => NotifyMe.sendMessage(url, token, message);
    }
}
"#;
    ([(header::CONTENT_TYPE, "text/javascript")], result)
}
