use std::collections::HashMap;

use rust_embed::RustEmbed;
use serde::Deserialize;

#[derive(RustEmbed)]
#[folder = "resources"]
struct Resources;

#[derive(Debug, Clone, Deserialize)]
pub struct FlashOptionAlgo {
  pub unlock_key: Vec<String>,
  pub key_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FlashAlgo {
  pub unlock_key: Vec<String>,
  pub key_path: String,
  pub lock_path: String,
  pub option: FlashOptionAlgo
}

#[derive(Debug, Clone, Deserialize)]
pub struct Algo {
  pub pattern: String,
  pub svd: String,
  pub flash: FlashAlgo
}

lazy_static::lazy_static! {
  pub static ref ALGOS: Vec<(regex::Regex, Algo, svd_parser::svd::Device)> = {
    let mut m = Vec::new();
    for resource in Resources::iter().filter(|x| x.starts_with("algo/")) {
      let resource = Resources::get(&resource).unwrap();
      let algo: Algo = serde_json::from_slice(&resource.data[..]).unwrap();
      let svd = svd_parser::parse(&String::from_utf8_lossy(&Resources::get(&algo.svd).unwrap().data)).unwrap();
      m.push((regex::Regex::new(&algo.pattern).unwrap(), algo, svd));
    }
    m
  };
}