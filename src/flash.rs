use std::{fs::File, io::Read, path::Path, time::Duration};

use grapple_bundle_lib::Index;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use probe_rs::{flashing::{DownloadOptions, FlashProgress, ProgressEvent}, probe::list::Lister, MemoryInterface, Permissions, Session};
use serde::Deserialize;
use svd_parser::svd::Device;
use zip::ZipArchive;

use crate::resources::ALGOS;

pub fn get_svd_field(path: &str, svd: &Device) -> anyhow::Result<(/* addr */ u64, /* offset */ u32, /* len */ u32)> {
  let mut path = path.split("/");
  let peripheral = svd.get_peripheral(path.next().ok_or(anyhow::anyhow!("Path does not include peripheral"))?).ok_or(anyhow::anyhow!("Peripheral does not exist"))?;
  let register = peripheral.get_register(path.next().ok_or(anyhow::anyhow!("Path does not include register"))?).ok_or(anyhow::anyhow!("Register does not exist"))?;
  let field = register.get_field(path.next().ok_or(anyhow::anyhow!("Path does not include field"))?).ok_or(anyhow::anyhow!("Field does not exist"))?;
  
  Ok(( peripheral.base_address + register.address_offset as u64, field.bit_offset(), field.bit_width() ))
}

pub fn set_svd_field(core: &mut probe_rs::Core<'_>, field: (u64, u32, u32), value: u32) -> anyhow::Result<()> {
  let (addr, offset, len) = field;
  if len == 32 && offset == 0 {
    // Just write it directly
    core.write_32(addr, &[value])?;
  } else {
    if offset + len > 32 {
      anyhow::bail!("Can't write a field that's not aligned :(")
    } else {
      let mut current = [0u32; 1];
      core.read_32(addr, &mut current[..])?;
      
      let mask = (2u64.pow(len) - 1) as u32;

      current[0] &= !(mask << offset);
      current[0] |= (value & mask) << offset;

      core.write_32(addr, &current[..])?;
    }
  }
  Ok(())
}

fn init_progress_bar(bar: &ProgressBar) {
    let style = bar.style().progress_chars("##-");
    bar.set_style(style);
    bar.enable_steady_tick(Duration::from_millis(100));
    bar.reset_elapsed();
}

pub fn flash<T: Read>(session: &mut Session, read: &mut T) -> anyhow::Result<()> {
  let multi_progress = MultiProgress::new();

  // Copied from probe-rs
  let style = ProgressStyle::default_bar()
                    .tick_chars("⠁⠁⠉⠙⠚⠒⠂⠂⠒⠲⠴⠤⠄⠄⠤⠠⠠⠤⠦⠖⠒⠐⠐⠒⠓⠋⠉⠈⠈✔")
                    .progress_chars("--")
                    .template("{msg:.green.bold} {spinner} [{elapsed_precise}] [{wide_bar}] {bytes:>8}/{total_bytes:>8} @ {bytes_per_sec:>10} (eta {eta:3})").unwrap();

  let erase_progress = multi_progress.add(ProgressBar::new(0));
  erase_progress.set_style(style.clone());
  erase_progress.set_message("      Erasing");

  let program_progress = multi_progress.add(ProgressBar::new(0));
  program_progress.set_style(style);
  program_progress.set_message("  Programming");

  let progress = FlashProgress::new(move |event| match event {
    ProgressEvent::Initialized { flash_layout } => {
        let total_sector_size: u64 = flash_layout.sectors().iter().map(|s| s.size()).sum();
        erase_progress.set_length(total_sector_size);
    }
    ProgressEvent::StartedProgramming { length } => {
        init_progress_bar(&program_progress);
        program_progress.set_length(length);
    }
    ProgressEvent::StartedErasing => {
        init_progress_bar(&erase_progress);
    }
    ProgressEvent::StartedFilling => {  }
    ProgressEvent::PageProgrammed { size, .. } => {
        program_progress.inc(size as u64);
    }
    ProgressEvent::SectorErased { size, .. } => {
        erase_progress.inc(size);
    }
    ProgressEvent::PageFilled { size, .. } => { }
    ProgressEvent::FailedErasing => {
        erase_progress.abandon();
        program_progress.abandon();
    }
    ProgressEvent::FinishedErasing => {
        erase_progress.finish();
    }
    ProgressEvent::FailedProgramming => {
        program_progress.abandon();
    }
    ProgressEvent::FinishedProgramming => {
        program_progress.finish();
    }
    ProgressEvent::FailedFilling => { }
    ProgressEvent::FinishedFilling => { }
    ProgressEvent::DiagnosticMessage { .. } => (),
  });

  let mut options = DownloadOptions::default();
  options.progress = Some(progress);
  options.keep_unwritten_bytes = false;
  options.dry_run = false;
  options.do_chip_erase = false;
  options.disable_double_buffering = false;
  options.verify = true;

  let mut loader = session.target().flash_loader();
  loader.load_elf_data(read)?;

  loader.commit(session, options)?;
  Ok(())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action")]
#[serde(rename_all = "snake_case")]
pub enum FlashAction {
  UnlockFlash,
  LockFlash,
  UnlockOptBytes,
  FlashBootloader,
  FlashFirmware,
  SetField {
    path: String,
    value: u32,
  },
}

impl FlashAction {
  pub fn run(&self, chip: &str, index: &Index, archive: &mut ZipArchive<File>, session: &mut Session) -> anyhow::Result<()> {
    let mut core = session.core(0)?;
    
    let (_, algo, svd) = ALGOS.iter().find(|x| x.0.is_match(chip)).ok_or(anyhow::anyhow!("Unknown Algorithm for Chip: {}", chip))?;

    match self {
      FlashAction::UnlockFlash => {
        println!("[x] UNLOCKING FLASH");
        let key_field = get_svd_field(&algo.flash.key_path, svd)?;
        for key in &algo.flash.unlock_key {
          set_svd_field(&mut core, key_field, u32::from_str_radix(key.trim_start_matches("0x"), 16)?)?;
        }
      },
      FlashAction::LockFlash => {
        println!("[x] LOCKING FLASH");
        let lock_field = get_svd_field(&algo.flash.lock_path, svd)?;
        set_svd_field(&mut core, lock_field, 0b1)?;
      },
      FlashAction::UnlockOptBytes => {
        println!("[x] UNLOCKING OPTION BYTES");
        let key_field = get_svd_field(&algo.flash.option.key_path, svd)?;
        for key in &algo.flash.option.unlock_key {
          set_svd_field(&mut core, key_field, u32::from_str_radix(key.trim_start_matches("0x"), 16)?)?;
        }
      },
      FlashAction::FlashBootloader => {
        println!("[x] FLASHING BOOTLOADER");
        drop(core);
        let mut f = archive.by_name(&index.bootloader)?;
        flash(session, &mut f)?;
        println!("... Done!");
      },
      FlashAction::FlashFirmware => {
        println!("[x] FLASHING FIRMWARE");
        drop(core);
        let mut f = archive.by_name(&index.firmware)?;
        flash(session, &mut f)?;

        println!("... Done!");
      },
      FlashAction::SetField { path, value } => {
        println!("[x] SETTING FIELD {}", path);
        let field = get_svd_field(path, svd)?;
        set_svd_field(&mut core, field, *value)?;
      },
    }
    Ok(())
  }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FlashConfig {
  #[serde(default)]
  pub procedure: Vec<FlashAction>,
}



pub fn flash_bundle(bundle: &Path, chip: &str) -> anyhow::Result<()> {
  // Step 1 - open bundle, read index, read config
  let file = File::open(bundle)?;
  let mut archive = zip::ZipArchive::new(file)?;

  let mut index_file = archive.by_name("index")?;
  let mut index_content = String::new();
  index_file.read_to_string(&mut index_content)?;
  let index: Index = serde_json::from_str(&index_content)?;
  drop(index_file);

  let mut config_file = archive.by_name(&index.config)?;
  let mut config_content = String::new();
  config_file.read_to_string(&mut config_content)?;
  let config: FlashConfig = serde_json::from_str(&config_content)?;
  drop(config_file);

  // Step 2 - open probe
  let lister = Lister::new();
  let probe = lister.list_all().first().ok_or(anyhow::anyhow!("No probes found!"))?.open(&lister)?;

  let mut session = probe.attach(chip, Permissions::default())?;

  // Step 3 - run routine
  for action in config.procedure.iter() {
    action.run(chip, &index, &mut archive, &mut session)?;
  }

  Ok(())
}