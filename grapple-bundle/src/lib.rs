use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
  pub firmware: String,
  pub firmware_update: String,
  pub firmware_update_bin: String,
  pub bootloader: String,
  pub bootloader_update_bin: String,
  pub config: String,
  pub firmware_version: String,
  pub bootloader_version: String,
}