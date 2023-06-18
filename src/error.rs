/*
 * Copyright 2023 Alex Syrnikov <alex.syrnikov19@gmail.com>
 * SPDX-License-Identifier: Apache-2.0
 */

#[derive(thiserror::Error, Debug)]
pub enum BotError {
    // #[error("data store disconnected")]
    // Disconnect(#[from] io::Error),
    // #[error("the data for key `{0}` is not available")]
    // Redaction(String),
    // #[error("invalid header (expected {expected:?}, found {found:?})")]
    // InvalidHeader {
    //     expected: String,
    //     found: String,
    // },
    // #[error("unknown error {0}")]
    // Unknown(String),
    #[error(transparent)]
    SerdeJsonError( #[from] serde_json::error::Error),
    #[error(transparent)]
    HyperError( #[from] hyper::Error),
    #[error(transparent)]
    SqlxError( #[from] sqlx::error::Error),
    #[error("Unspecified ring error")]
    RingError(),
    #[error(transparent)]
    AxumHttpError( #[from] axum::http::Error),
}
// #[derive(thiserror::Error, Debug)]
// #[error("error with message {msg}")]
// pub struct ErrorWithMessage {
//     msg: String,
//     #[source]
//     source: BotError
// }
/*
 let _file = File::open("foo.txt").map_err(|err| {
        ErrorWithMessage { msg: format!("file name foo.txt"), source: err.into() }
    })
    .map_err(|err| {
        println!( "error {err}" );
        println!( "error to_string() {}", err.to_string() );
        println!( "error {err:?}" );
        err
    })?;
 */
