#![doc = include_str!("../README.md")]
#![doc(html_favicon_url = "https://photino.github.io/zino-docs-zh/assets/zino-logo.png")]
#![doc(html_logo_url = "https://photino.github.io/zino-docs-zh/assets/zino-logo.svg")]
#![forbid(unsafe_code)]
#![feature(doc_auto_cfg)]
#![feature(lazy_cell)]

#[cfg(feature = "cache")]
pub mod cache;
#[cfg(feature = "format")]
pub mod format;
