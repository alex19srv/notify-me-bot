/*
 * Copyright 2023 Alex Syrnikov <alex.syrnikov19@gmail.com>
 * SPDX-License-Identifier: Apache-2.0
 */

use once_cell::sync::OnceCell;
use ring::rand::SecureRandom;
use crate::error::BotError;

static SYS_RANDOM: OnceCell<ring::rand::SystemRandom> = OnceCell::new();
#[inline]
pub fn gen_random(buffer: &mut [u8]) -> Result<(),BotError>
{
    let sys_random = unsafe { SYS_RANDOM.get_unchecked() };
    sys_random.fill(buffer).map_err(|_err| {BotError::RingError()})?;

    return Ok(());
}

pub fn init() -> Result<(), BotError>
{
    let sys_random = ring::rand::SystemRandom::new();
    SYS_RANDOM.set(sys_random).unwrap();

    return Ok(());
}
