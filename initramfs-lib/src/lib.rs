#![no_std]

use alloc::{format, vec};
use alloc::string::{String, ToString};
use rusl::platform::FilesystemType;
use rusl::unistd::{mount, swapon, unmount};
use tiny_std::io::Read;
use tiny_std::process::{Command, Stdio};
use unix_print::unix_eprintln;
use crate::error::{Error, Result};

mod error;

extern crate alloc;

pub fn full_init(cfg: &Cfg) -> Result<()> {
    mount_pseudo_filesystems()
        .map_err(|e| Error::App(format!("Failed to mount pseudo filesystems {e:?}")))?;
    mount_user_filesystems(cfg)
        .map_err(|e| Error::App(format!("Failed to mount user filesystems {e:?}")))?;
    try_unmount()
        .map_err(|e| Error::App(format!("Failed to unmount pseudo filesystems {e:?}")))?;
    Ok(())
}

pub fn mount_pseudo_filesystems() -> Result<()> {
    mount::<_, _, &'static str>("proc\0", "/proc\0", FilesystemType::Proc, 0, None)
        .map_err(|e| Error::MountPseudo(format!("Failed to mount proc fs: {e}")))?;
    mount::<_, _, &'static str>("sys\0", "/sysfs\0", FilesystemType::Sysfs, 0, None)
        .map_err(|e| Error::MountPseudo(format!("Failed to mount sysfs: {e}")))?;
    mount::<_, _, &'static str>("dev\0", "/sysfs\0", FilesystemType::Sysfs, 0, None)
        .map_err(|e| Error::MountPseudo(format!("Failed to mount sysfs: {e}")))?;
    Ok(())
}

pub fn mount_user_filesystems(cfg: &Cfg) -> Result<()> {
    let parts = get_partitions(cfg)
        .map_err(|e| Error::Mount(format!("Failed to find partitions {e:?}")))?;
    mount::<_, _, &'static str>(&parts.root, "/root/mnt\0", FilesystemType::Ext4, 0, None)
        .map_err(|e| Error::Mount(format!("Failed to mount root partition {} to /root/mnt: {e:?}", parts.root)))?;
    mount::<_, _, &'static str>(&parts.home, "/root/mnt/home\0", FilesystemType::Ext4, 0, None)
        .map_err(|e| Error::Mount(format!("Failed to mount home partition {} to /root/mnt/home: {e:?}", parts.home)))?;
    swapon(&parts.swap, 0)
        .map_err(|e| Error::Mount(format!("Failed to swapon {}: {e:?}", parts.swap)))?;
    Ok(())
}

#[cfg_attr(test, derive(Debug))]
pub struct Partitions {
    pub root: String,
    pub swap: String,
    pub home: String,
}

pub fn get_partitions(cfg: &Cfg) -> Result<Partitions> {
    let cmd = Command::new("blkid")
        .map_err(|e| Error::FindPartitions(format!("Failed to instantiate blkid command {e}")))?;
    let tgt = spawn_await_stdout(cmd, 4096)?;
    let mut root = None;
    let mut swap = None;
    let mut home = None;
    for line in tgt.lines() {
        // Dirty just checking contains, which essentially mean we also accept part-uuids since they
        // are on the same line.

        // /dev/nvme1n1p4: ...UUID=... etc
        if line.contains(&cfg.root_uuid) {
            let (part, _discard_rest) = line.split_once(':')
                .ok_or_else(|| Error::FindPartitions(format!("Failed to find root partition device name on blkid line that contains the specified uuid={}, line={line}", cfg.root_uuid)))?;
            root = Some(part.to_string())
        } else if line.contains(&cfg.swap_uuid) {
            let (part, _discard_rest) = line.split_once(':')
                .ok_or_else(|| Error::FindPartitions(format!("Failed to find swap partition device name on blkid line that contains the specified uuid={}, line={line}", cfg.swap_uuid)))?;
            swap = Some(part.to_string())
        } else if line.contains(&cfg.home_uuid) {
            let (part, _discard_rest) = line.split_once(':')
                .ok_or_else(|| Error::FindPartitions(format!("Failed to find home partition device name on blkid line that contains the specified uuid={}, line={line}", cfg.home_uuid)))?;
            home = Some(part.to_string())
        }
    }
    Ok(Partitions {
        root: root.ok_or_else(|| Error::FindPartitions(format!("Failed to find root partition={} from blkid", cfg.root_uuid)))?,
        swap: swap.ok_or_else(|| Error::FindPartitions(format!("Failed to find swap partition={} from blkid", cfg.swap_uuid)))?,
        home: home.ok_or_else(|| Error::FindPartitions(format!("Failed to find home partition={} from blkid", cfg.home_uuid)))?,
    })
}

pub(crate) fn spawn_await_stdout(mut cmd: Command, buf_size: usize) -> Result<String> {
    let mut child = cmd.stdout(Stdio::MakePipe)
        .spawn()
        .map_err(|e| Error::Spawn(format!("Failed to spawn command {e}")))?;
    let res = child.wait()
        .map_err(|e| Error::Spawn(format!("Failed to wait for child to exit {e}")))?;
    if res != 0 {
        return Err(Error::Spawn(format!("Got bad exit code {res} from child")));
    }
    let mut buf = vec![0u8; buf_size];
    let mut stdout = child.stdout
        .ok_or_else(|| Error::Spawn("Failed to get child stdout handle".to_string()))?;
    let read_bytes = stdout.read(&mut buf)
        .map_err(|e| Error::Spawn(format!("Failed to read from child stdout handle {e}")))?;
    // Maybe don't double alloc here but who cares really
    String::from_utf8(buf[..read_bytes].to_vec())
        .map_err(|e| Error::Spawn(format!("Failed to convert child stdout to utf8 {e}")))
}

// This can fail without it necessarily being a problem
pub fn try_unmount() -> Result<()> {
    if let Err(e) = unmount("/proc\0") {
        unix_eprintln!("Failed to unmount proc fs: {e}");
    }
    if let Err(e) = unmount("/sys\0") {
        unix_eprintln!("Failed to unmount sysfs {e}");
    }
    if let Err(e) = unmount("/dev\0") {
        unix_eprintln!("Failed to unmount devtmpfs {e}");
    }
    Ok(())
}

pub struct Cfg {
    root_uuid: String,
    swap_uuid: String,
    home_uuid: String,
}

pub fn read_cfg(cfg_path: &str) -> Result<Cfg> {
    let content = tiny_std::fs::read_to_string(cfg_path)
        .map_err(|e| Error::Cfg(format!("Failed to read cfg at {cfg_path}: {e}")))?;
    let mut root_uuid = None;
    let mut swap_uuid = None;
    let mut home_uuid = None;
    for (ind, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
           continue;
        }
        // Allow comments
        if trimmed.starts_with("//") {
            continue;
        }
        let (key, value) = trimmed.split_once('=')
            .ok_or_else(|| Error::Cfg(format!("Found non empty line that doesn't contain '=' or starts with '//' [{ind}]: '{line}'")))?;
        match key {
            "root" => root_uuid = Some(value.to_string()),
            "home" => home_uuid = Some(value.to_string()),
            "swap" => swap_uuid = Some(value.to_string()),
            other => return Err(Error::Cfg(format!("Unrecognized key in config file {other} at [{ind}]: '{line}'"))),
        }
    }
    Ok(Cfg {
        root_uuid: root_uuid.ok_or_else(|| Error::Cfg(format!("No root uuid found in cfg at path {cfg_path}")))?,
        swap_uuid: swap_uuid.ok_or_else(|| Error::Cfg(format!("No swap uuid found in cfg at path {cfg_path}")))?,
        home_uuid: home_uuid.ok_or_else(|| Error::Cfg(format!("No home uuid found in cfg at path {cfg_path}")))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Needs your testing machine's disk uuids
    #[test]
    fn test_blkid() {
        let cfg = read_cfg("/home/gramar/code/rust/yubi-initramfs/initramfs.cfg").unwrap();
        let parts = get_partitions(&cfg).unwrap();
        unix_eprintln!("{parts:?}");
    }
}
