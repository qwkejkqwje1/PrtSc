pub mod config;
pub mod editor;
pub mod history;
pub mod image;
pub mod render;
pub mod stitch;
pub mod ui;
pub mod util;

/// Complete license text, embedded once for the single-file distribution.
pub const LICENSE_TEXT: &str = concat!(
    include_str!("../LICENSE"),
    "\n\n",
    include_str!("../THIRD-PARTY-NOTICES.txt")
);

pub mod google;

pub mod hosting;
pub mod video;

pub mod i18n;

pub mod unicode;

pub mod dock;
pub mod links;

pub mod hotkey;

pub mod damage;
pub mod text;

pub mod translate;
