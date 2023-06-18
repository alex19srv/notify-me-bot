/*
 * Copyright 2023 Alex Syrnikov <alex.syrnikov19@gmail.com>
 * SPDX-License-Identifier: Apache-2.0
 */

use std::{time::{SystemTime, Duration}, str::FromStr, result};
use sqlx::{
    migrate::MigrateDatabase
    ,sqlite::{
        Sqlite
        ,SqlitePool
        ,SqlitePoolOptions
        ,SqliteConnectOptions
    }
};
use once_cell::sync::OnceCell;
use crate::error::BotError;

static DB_POOL: OnceCell<SqlitePool> = OnceCell::new();

#[inline]
fn pool() -> &'static SqlitePool {
    unsafe { DB_POOL.get_unchecked() }
}
pub async fn init(db_path: &str) -> Result<(),BotError>
{
    let db_url = format!("sqlite://{}",db_path);
    if !Sqlite::database_exists(&db_url).await? {
        println!("Creating database {}", db_url);
        Sqlite::create_database(&db_url).await?;
        println!("db created");
    }
    let options = SqliteConnectOptions::from_str(format!("sqlite://{}",db_path).as_str())?
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .foreign_keys(false);
    let db_pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with( options )
        .await.expect("Failed open Sqlite DB");
    DB_POOL.set(db_pool).unwrap();

    create_schema().await?;

    return Ok(());
}
async fn create_schema() -> Result<(), BotError> {
    let query =
    "CREATE TABLE IF NOT EXISTS sessions
    (
        token         BLOB NOT NULL PRIMARY KEY,
        chat_id    INTEGER NOT NULL,
        created_at INTEGER NOT NULL DEFAULT 0
    );";
    sqlx::query(query).execute(pool()).await?;

    Ok(())
}
/// current timestamp
fn unix_time_current() -> i64 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or(Duration::ZERO).as_secs() as i64
}

pub async fn create_session( token: &[u8], chat_id: i64 ) -> Result<(),BotError>
{
    sqlx::query(
        "DELETE FROM sessions \
        WHERE chat_id = $1"
    ).bind(chat_id).execute(pool()).await?;

    sqlx::query(
        "INSERT INTO sessions(token, chat_id, created_at)
        VALUES ($1,$2,$3)")
        .bind(&token).bind(chat_id).bind(unix_time_current())
        .execute(pool())
        .await?;

    return Ok(());
}

pub async fn delete_session( chat_id: i64 ) -> Result<(),BotError>
{
    sqlx::query("DELETE FROM sessions WHERE chat_id = $1")
        .bind(&chat_id).execute(pool())
        .await?;

    return Ok(());
}

pub async fn find_chat_by_token(token: &[u8] ) -> Result<Option<i64>,BotError>
{
    let row = sqlx::query_as::<_,(i64,)>(
        "SELECT chat_id
        FROM    sessions
        WHERE   token = $1"
    )
        .bind(token).fetch_optional(pool())
        .await?;

    return Ok(row.map( |(id,)| {id} ));
}
pub async fn find_token_by_chat(chat_id: i64 ) -> Result<Option<Vec<u8>>,BotError>
{
    let row = sqlx::query_as::<_,(Vec<u8>,)>(
        "SELECT token
        FROM    sessions
        WHERE   chat_id = $1"
    )
        .bind(chat_id).fetch_optional(pool())
        .await?;

    return Ok(row.map( |(token,)| {token} ));
}
