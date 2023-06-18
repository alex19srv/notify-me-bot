/*
 * Copyright 2023 Alex Syrnikov <alex.syrnikov19@gmail.com>
 * SPDX-License-Identifier: Apache-2.0
 */

use std::sync::Arc;
use base64::Engine;
use base64::engine::general_purpose::NO_PAD;
use hyper::{client::HttpConnector, body::to_bytes, client, Body};
use hyper_rustls::{ConfigBuilderExt, HttpsConnector};

use crate::error::BotError;

pub mod api_type;
mod updates_handler;
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;
use crate::random;

static TG_BOT: OnceCell<TelegramBot> = OnceCell::new();
#[inline]
fn bot() -> &'static TelegramBot {
    unsafe { TG_BOT.get_unchecked() }
}
#[derive(PartialEq,Debug)]
enum PollingMode {
    Unknown,
    Polling,
    Webhook,
}
#[derive(Debug)]
pub struct TelegramBot {
    token: String,
    https_client: HttpsClient,
    webhook_url: String,
    webhook_token: String,
    polling_mode: Arc<Mutex<PollingMode>>
}

impl TelegramBot {
    pub async fn init(token: &str, webhook_url: &str) -> Result<(),BotError> {
        random::init();
        {
            let https_client: HttpsClient = create_https_client();

            let mut webtoken_buf: [u8; 32] = [0; 32];
            random::gen_random(&mut webtoken_buf[..])?;

            let token_alphabet =
                base64::alphabet::Alphabet::new("-_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789")
                    .unwrap();
            let token_engine = base64::engine::GeneralPurpose::new(&token_alphabet, NO_PAD);
            let webhook_token = token_engine.encode(&webtoken_buf);

            let bot = TelegramBot {
                token: token.to_string()
                ,https_client
                ,webhook_url: webhook_url.to_string()
                ,webhook_token
                ,polling_mode: Arc::new(Mutex::new(PollingMode::Unknown))};
            TG_BOT.set(bot).unwrap();
        }

        updates_handler::init();
        bot().set_my_commands().await?;
        Self::set_mode_polling().await?;

        return Ok(());
    }
    pub async fn send_message(
        chat_id: i64, text: &str
    ) -> Result<api_type::ApiMessage, BotError>
    {
        return bot().send_message_imp( chat_id, text ).await;
    }
    pub async fn set_mode_webhook() -> Result<(),BotError> {
        return bot().set_mode_webhook_impl().await;
    }
    async fn set_mode_webhook_impl(&self) -> Result<(),BotError> {
        let mut lock_guard = self.polling_mode.lock().await;
        if *lock_guard == PollingMode::Webhook {
            return Ok(());
        }

        self.set_webhook().await?;
        *lock_guard = PollingMode::Webhook;

        return Ok(());
    }
    pub async fn set_mode_polling() -> Result<(),BotError> {
        return bot().set_mode_polling_impl().await;
    }
    async fn set_mode_polling_impl(&self) -> Result<(),BotError> {
        let mut lock_guard = self.polling_mode.lock().await;
        if *lock_guard == PollingMode::Polling {
            return Ok(());
        }

        self.query("deleteWebhook").await?;
        tokio::spawn( async {
            updates_handler::poll_updates().await.unwrap();
        });
        *lock_guard = PollingMode::Polling;

        return Ok(());
    }
    pub async fn handle_webhook_update(
        webhook_token: &str, api_update: api_type::ApiUpdate
    ) -> Result<(), BotError> {
        let current_token: &str = &bot().webhook_token;
        if webhook_token != current_token {
            println!("handle_webhook_update() got incorrect token\n{webhook_token} expected\n{current_token}");
            return Ok(());
        }
        return updates_handler::handle_update( api_update ).await;
    }

    async fn get_me(&self) -> Result<api_type::ApiUser, BotError> {
        let json_value = self.query("getMe").await?;
        let user: api_type::ApiUser = serde_json::from_value(json_value)?;

        Ok(user)
    }
    async fn set_my_commands(&self) -> Result<(), BotError> {
        let mut params = api_type::SetMyCommandsParams::default();
        for cmd in updates_handler::CMD_LIST {
            let command = api_type::ApiBotCommand {
                command: cmd.name,
                description: cmd.description,
            };
            params.commands.push(command);
        }
        let params_str = serde_json::to_string(&params)?;

        let json_value = self.query_with_params("setMyCommands", &params_str).await?;
        // println!("setMyCommands returned {json_value:?}");

        return Ok(());
    }

    async fn set_webhook(&self) -> Result<(), BotError>
    {
        let mut params = api_type::SetWebhookParams::default();
        params.url = &self.webhook_url;
        params.secret_token = &self.webhook_token;
        let params_str = serde_json::to_string(&params)?;

        let json_value = self.query_with_params("setWebhook", &params_str).await?;
        println!("setWebhook returned {json_value:?}");

        return Ok(());
    }

    async fn get_updates(offset: i64) -> Result<Vec<api_type::ApiUpdate>, BotError> {
        // println!("get_updates() with offset {offset}");
        return bot().get_updates_impl(offset).await;
    }
    async fn get_updates_impl(&self, offset: i64) -> Result<Vec<api_type::ApiUpdate>, BotError> {
        let mut update_params = api_type::GetUpdatesParams::default();
        update_params.timeout = Some(1024);
        update_params.offset = Some(offset);
        let params_str = serde_json::to_string(&update_params)?;

        let json_value = self.query_with_params("getUpdates", &params_str).await?;
        // println!("get updates got {json_value:?}");

        let update_list: Vec<api_type::ApiUpdate> = serde_json::from_value(json_value)?;
        // println!("update list size = {}", update_list.len());
        // println!("{update_list:?}");

        Ok(update_list)
    }
    async fn send_message_imp(&self, chat_id: i64, text: &str)
        -> Result<api_type::ApiMessage, BotError>
    {
        let send_message_params = api_type::SendMessageParams {
            chat_id, text: &text
        };
        let params_str = serde_json::to_string(&send_message_params)?;

        let json_value = self.query_with_params("sendMessage", &params_str).await?;
        // println!("send message got {json_value:?}");

        let api_message: api_type::ApiMessage = serde_json::from_value(json_value)?;
        // println!("api message from send message = {:?}", api_message);

        Ok(api_message)
    }

    async fn query(&self, method_name: &str)
        -> Result<serde_json::Value, BotError>
    {
        return self.query_with_params(method_name, "{}").await;
    }
    async fn query_with_params(
        &self, method_name: &str, params_str: &str
    ) -> Result<serde_json::Value, BotError>
    {
        let url = format!("https://api.telegram.org/bot{}/{}", self.token, method_name);
        return self.https_query(&url, params_str).await;
    }

    async fn https_query(&self,url: &str, params_json_str: &str)
        -> Result<serde_json::Value, BotError>
    {
        let req = hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri(url)
            .header("content-type", "application/json")
            .body(Body::from(params_json_str.to_string()))?;

        let resp = self.https_client.request(req).await?;
        // println!("Status:\n{}", resp.status());
        // println!("Headers:\n{:#?}", resp.headers());

        let body: Body = resp.into_body();
        let body = to_bytes(body).await?;
        // println!("Body:\n{}", String::from_utf8_lossy(&body));

        let result: api_type::QueryResult = serde_json::from_slice(&body)?;
        if result.ok == false {
            tracing::warn!("Query error:\nUrl: {url}\nBody: {params_json_str}\nreturned error:{result:?}");
        }
        // FIXME: return error on non 2xx status
        // println!("QueryResult object {result:?}");

        return Ok(result.result.unwrap());
    }
}


type HttpsClient = hyper::Client<HttpsConnector<HttpConnector>>;

fn create_https_client() -> HttpsClient {
    let tls = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_native_roots()
        .with_no_client_auth();
    let https_connector = hyper_rustls::HttpsConnectorBuilder::new()
        .with_tls_config(tls)
        .https_only()
        // .https_or_http()
        .enable_http1()
        .build();

    let client = client::Client::builder().build(https_connector);

    return client;
}
