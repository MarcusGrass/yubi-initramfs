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
        "--list-partitions" => {
            let partitions = initramfs_lib::get_partitions(&cfg)
                .map_err(|e| {
                    unix_eprintln!("Failed to get partitions: {e:?}");
                    1
                })?;
            unix_println!("Root: {}\nSwap: {}\nHome: {}", partitions.root, partitions.swap, partitions.home);
            Ok(())
        }
        "--init" => {
            initramfs_lib::full_init(&cfg)
                .map_err(|e| {
                    unix_eprintln!("Failed init full {e:?}");
                    1
                })
        }
        s => {
            unix_eprintln!("Unrecognized argument {s}");
            Err(1)
        }
    }
}

