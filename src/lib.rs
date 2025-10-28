#![allow(clippy::single_component_path_imports)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(html_favicon_url = "../3.ico")]
#![doc(html_logo_url = "../3.ico")]

//! [![D+VINE Logo](../logo.jpg)](https://github.com/master-g/dvine-rs.git)
//!
//! `dvine-rs` is a project that aims to revive an old game and bring it to modern platforms using Rust.
//!
pub use dvine_internal::*;

#[cfg(all(feature = "dynamic_linking", not(target_family = "wasm")))]
#[allow(unused_imports)]
use dvine_dylib;
