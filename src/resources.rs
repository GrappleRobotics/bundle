use std::{collections::HashMap, io::Read};

use rust_embed::RustEmbed;
use serde::Deserialize;
use zip::ZipArchive;

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
    let svd_resource = Resources::get("svd.zip").unwrap();
    let mut zip = ZipArchive::new(std::io::Cursor::new(&svd_resource.data[..])).unwrap();
    for resource in Resources::iter().filter(|x| x.starts_with("algo/")) {
      let resource = Resources::get(&resource).unwrap();
      let algo: Algo = serde_json::from_slice(&resource.data[..]).unwrap();
      let mut s = String::new();
      zip.by_name(&algo.svd).unwrap().read_to_string(&mut s).unwrap();
      let svd = svd_parser::parse(&s).unwrap();
      m.push((regex::Regex::new(&algo.pattern).unwrap(), algo, svd));
    }
    m
  };
}