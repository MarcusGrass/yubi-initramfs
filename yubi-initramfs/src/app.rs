use unix_print::{unix_eprintln, unix_println};
use initramfs_lib::read_cfg;

/// Some references [Gentoo custom initramfs](https://wiki.gentoo.org/wiki/Custom_Initramfs)
/// [Boot kernel without bootloader](https://tecporto.pt/wiki/index.php/Booting_the_Linux_Kernel_without_a_bootloader)
pub(crate) fn main_loop() -> Result<(), i32> {
    let mut args = tiny_std::env::args();
    let _self = args.next();
    let cfg_path = args.next()
        .ok_or_else(|| {
            unix_eprintln!("No cfg path supplied, required as first argument");
            1
        })?
        .map_err(|e| {
            unix_eprintln!("First arg not parseable as utf8: {e}");
            1
        })?;
    let command = args.next()
        .ok_or_else(|| {
            unix_eprintln!("Missing command argument");
            1
        })?
        .map_err(|e| {
            unix_eprintln!("Command arg not parseable as utf8: {e}");
            1
        })?;
    let cfg = read_cfg(cfg_path)
        .map_err(|e| {
            unix_eprintln!("Failed to read cfg: {e:?}");
            1
        })?;
    match command {
        "--list-partitions" | "-l" => {
            let partitions = initramfs_lib::get_partitions(&cfg)
                .map_err(|e| {
                    unix_eprintln!("Error: Failed to get partitions: {e:?}");
                    1
                })?;
            unix_println!("Successfully found partitions.\nRoot: {}\nSwap: {}\nHome: {}", partitions.root, partitions.swap, partitions.home);
            Ok(())
        }
        "--mount-pseudo" | "-p" => {
            initramfs_lib::mount_pseudo_filesystems()
                .map_err(|e| {
                    unix_eprintln!("Error: Failed to mount pseudo filesystems {e:?}");
                    1
                })?;
            unix_println!("Successfully mounted pseudo filesystem.");
            Ok(())
        }
        "--run-mdev" | "-m" => {
            initramfs_lib::run_mdev()
                .map_err(|e| {
                    unix_eprintln!("Error: Failed to run mdev {e:?}");
                    1
                })?;
            unix_println!("Successfully ran mdev.");
            Ok(())
        }
        "--mount-user" | "-u" => {
            initramfs_lib::mount_user_filesystems(&cfg).map_err(|e| {
                unix_eprintln!("Error: Failed to mount user filesystems using cfg  at path {cfg:?}: {e:?}");
                1
            })?;
            Ok(())
        }
        "--init" => {
            initramfs_lib::full_init(&cfg)
                .map_err(|e| {
                    unix_eprintln!("Error: Failed init full {e:?}");
                    1
                })?;
            unix_println!("Successfully ran init setup");
            Ok(())
        }
        s => {
            unix_eprintln!("Unrecognized argument {s}");
            Err(1)
        }
    }
}

