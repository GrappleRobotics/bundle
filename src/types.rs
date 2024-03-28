use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Index {
  pub firmware: String,
  pub firmware_update: String,
  pub firmware_update_bin: String,
  pub bootloader: String,
  pub bootloader_update_bin: String,
  pub svd: String,
  pub config: String,
}