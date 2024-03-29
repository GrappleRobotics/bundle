use std::{fs::File, io::{Read, Seek, Write}, path::Path};

use grapple_bundle::Index;
use object::{Object, ObjectSection, ObjectSegment};
use zip::{write::FileOptions, ZipWriter};

fn filename(path: &Path) -> anyhow::Result<String> {
  path.file_name().map(|x| x.to_string_lossy().to_string()).ok_or(anyhow::anyhow!("No file name!"))
}

fn read_version(obj: &object::File<'_>) -> anyhow::Result<String> {
   match obj.section_by_name(".metadata") {
    Some(meta) => {
      // meta.data()?
      let (addr, str_len) = match obj.endianness() {
        object::Endianness::Little => {
          (
            u32::from_le_bytes([ meta.data()?[4], meta.data()?[5], meta.data()?[6], meta.data()?[7] ]),
            u32::from_le_bytes([ meta.data()?[8], meta.data()?[9], meta.data()?[10], meta.data()?[11] ]),
          )
        },
        object::Endianness::Big => {
          (
            u32::from_be_bytes([ meta.data()?[4], meta.data()?[5], meta.data()?[6], meta.data()?[7] ]),
            u32::from_be_bytes([ meta.data()?[8], meta.data()?[9], meta.data()?[10], meta.data()?[11] ]),
          )
        },
      };
      for section in obj.sections() {
        if section.address() <= addr.into() && (section.address() + section.data()?.len() as u64) > (addr + str_len) as u64 {
          let offset = addr as usize - section.address() as usize;
          let version_str = &section.data()?[offset..offset+str_len as usize];
          
          return Ok(String::from_utf8_lossy(version_str).to_string());
        }
      }
    },
    None => anyhow::bail!("No metadata section. Is this a firmware file?"),
  };
  anyhow::bail!("No version found!")
}

fn gen_firmware_update_elf(data: &[u8]) -> anyhow::Result<(String, Vec<u8>)> {
  let obj = object::File::parse(&data[..])?;
  let version = read_version(&obj)?;

  let mut builder = object::build::elf::Builder::read32(&data[..])?;
  for section in builder.sections.iter_mut() {
    if section.name.to_string() == ".firmware_flag" {
      match &mut section.data {
        object::build::elf::SectionData::Data(dat) => {
          dat.to_mut().copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF])
        },
        _ => ()
      }
    }
  }

  let mut out = vec![];

  builder.write(&mut out)?;

  Ok((version, out))
}

fn convert_to_bin(data: &[u8]) -> anyhow::Result<(String, Vec<u8>)> {
  let obj = object::File::parse(&data[..])?;
  let version = read_version(&obj)?;

  let mut out_data = vec![];

  let mut offset = None;

  for segment in obj.segments() {
    let data = segment.data()?;
    if !data.is_empty() {
      // println!("{:x?}", segment.address());
      match offset {
        None => {
          out_data.extend_from_slice(data);
        },
        Some(x) => {
          let pad = segment.address() - x;
          out_data.extend(vec![0x00; pad as usize]);
          out_data.extend_from_slice(data);
        }
      }
      offset = Some(segment.address() + data.len() as u64);
    }
  }

  Ok((version, out_data))
}

fn write_file_to_zip<T: Write + Seek>(zip: &mut ZipWriter<T>, file: &Path) -> anyhow::Result<()> {
  let options = FileOptions::default()
    .unix_permissions(0o755);
  zip.start_file(file.file_name().map(|x| x.to_string_lossy()).ok_or(anyhow::anyhow!("No file name!"))?, options)?;
  let mut f = File::open(file)?;
  let mut buffer = Vec::new();
  f.read_to_end(&mut buffer)?;
  zip.write_all(&buffer)?;
  Ok(())
}

pub fn build_bundle(output: &Path, firmware: &Path, bootloader: &Path, config: &Path, lasercan_rev1_bootloader_check: bool) -> anyhow::Result<()> {
  let file = File::create(output)?;
  let mut zip = ZipWriter::new(file);

  write_file_to_zip(&mut zip, firmware)?;
  write_file_to_zip(&mut zip, bootloader)?;
  write_file_to_zip(&mut zip, config)?;
  
  let firmware_bytes = std::fs::read(firmware)?;
  let bootloader_bytes = std::fs::read(bootloader)?;

  // TODO: LaserCAN Check

  let (version, new_elf) = gen_firmware_update_elf(&firmware_bytes)?;
  let update_elf_name = format!("{}-{}-update.elf", &filename(firmware)?, version);
  zip.start_file(&update_elf_name, FileOptions::default().unix_permissions(0o755))?;
  zip.write_all(&new_elf[..])?;
  
  let (version, firmware_bin) = convert_to_bin(&new_elf[..])?;
  let update_bin_name = format!("{}-{}-update.grplfw", &filename(firmware)?, version);
  zip.start_file(&update_bin_name, FileOptions::default().unix_permissions(0o755))?;
  zip.write_all(&firmware_bin[..])?;

  let (bootloader_version, bootloader_bin) = convert_to_bin(&bootloader_bytes)?;
  let bootloader_update_bin_name = format!("{}-{}.grplbt", &filename(bootloader)?, bootloader_version);
  zip.start_file(&bootloader_update_bin_name, FileOptions::default().unix_permissions(0o755))?;
  zip.write_all(&bootloader_bin[..])?;

  let idx = Index { 
    firmware: filename(firmware)?,
    firmware_update: update_elf_name,
    firmware_update_bin: update_bin_name,
    bootloader: filename(bootloader)?,
    bootloader_update_bin: bootloader_update_bin_name,
    config: filename(config)?,
    firmware_version: version,
    bootloader_version: bootloader_version,
  };
  
  let idx_str = serde_json::to_string(&idx)?;
  zip.start_file("index", FileOptions::default().unix_permissions(0o755))?;
  zip.write_all(idx_str.as_bytes())?;

  zip.finish()?;
  Ok(())
}