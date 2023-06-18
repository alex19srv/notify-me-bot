/*
 * Copyright 2023 Alex Syrnikov <alex.syrnikov19@gmail.com>
 * SPDX-License-Identifier: Apache-2.0
 */

use std::time::SystemTime;

pub struct AppState {
    pub prev_query_time: SystemTime,
}
