#![allow(unused_imports)]
#![allow(clippy::too_many_arguments)]

extern crate serde_repr;
extern crate serde;
extern crate serde_json;
extern crate url;
extern crate reqwest;

pub mod apis;
pub mod models;
#[cfg(feature = "extras")]
pub mod websocket;
#[cfg(feature = "extras")]
pub mod websocket_resilient;
