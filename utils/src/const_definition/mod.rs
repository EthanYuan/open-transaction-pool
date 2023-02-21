pub mod devnet;

use once_cell::sync::OnceCell;

pub static CKB_URI: OnceCell<String> = OnceCell::new();
