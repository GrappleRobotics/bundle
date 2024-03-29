use std::{collections::HashMap, fs::File, io::Read};

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

    let svd_path_parent = home::home_dir().expect("Couldn't get home directory").join(".grapple").join("bundle");
    let svd_path = svd_path_parent.join("svd.zip");

    if !svd_path.exists() {
      println!("[x] DOWNLOADING SVDs");
      let mut resp = reqwest::blocking::get("https://github.com/modm-io/cmsis-svd-stm32/archive/refs/heads/main.zip").unwrap();
      std::fs::create_dir_all(svd_path_parent).unwrap();
      let mut f = std::fs::File::create(&svd_path).unwrap();
      std::io::copy(&mut resp, &mut f).unwrap();
      println!("...Done");
    }

    let svd_zip_file = File::open(svd_path).unwrap();
    let mut zip = ZipArchive::new(svd_zip_file).unwrap();
    for resource in Resources::iter().filter(|x| x.starts_with("algo/")) {
      let resource = Resources::get(&resource).unwrap();
      let algo: Algo = serde_json::from_slice(&resource.data[..]).unwrap();
      let mut s = String::new();
      zip.by_name(&format!("cmsis-svd-stm32-main/{}", algo.svd)).unwrap().read_to_string(&mut s).unwrap();
      let svd = svd_parser::parse(&s).unwrap();
      m.push((regex::Regex::new(&algo.pattern).unwrap(), algo, svd));
    }
    m
  };
}