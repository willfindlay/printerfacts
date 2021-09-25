// SPDX-License-Identifier: MIT
//
// Distributed printer facts in Rust, inspired by Christine Dodrill.
// Copyright (c) 2021  William Findlay
//
// September 25, 2021  William Findlay  Created this.

mod fairings;

pub use fairings::Counter;

/// Get the hostname and return it as a string
pub async fn get_hostname() -> String {
    gethostname::gethostname().to_string_lossy().into()
}
