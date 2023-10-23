use std::{env, fs};
use std::path::{Path, PathBuf};
use sysinfo::{DiskExt, System, SystemExt};
use std::process::{Command, exit}; //:sob:
use sys_mount::{
    Mount,
    MountFlags,
    SupportedFilesystems,
    Unmount,
    UnmountFlags
};
// test comment for commit
fn recovery() {
    let recovery_dir = "/sysroot/.recovery";
    let roots_dir = "/sysroot/roots/.recovery";
    println!("foxmount:  Unmounting overlays");
    sys_mount::unmount("/sysroot/usr", UnmountFlags::DETACH)
        .expect("unable to unmount /usr");
    sys_mount::unmount("/sysroot/etc", UnmountFlags::DETACH)
        .expect("unable to unmount /etc");
    sys_mount::unmount("/sysroot/var", UnmountFlags::DETACH)
        .expect("unable to unmount /var");

    println!("foxmount: Mounting recovery");
    match fs::create_dir("/sysroot/roots/.recovery") {
        Ok(_) => {}
        Err(_) => {}
    }
    let rec_res = Mount::builder()
        .fstype("tmpfs")
        .mount("tmpfs", "/sysroot/roots/.recovery");
    std::env::set_current_dir("/sysroot/roots/.recovery")
        .expect("how.");
    if let Ok(entries) = fs::read_dir(recovery_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let d = entry.file_name();
                let d_str = d.to_string_lossy();

                let lowerdir = format!("{}/{}:{}/{}", recovery_dir, d_str, recovery_dir, d_str);
                let upperdir = format!("{}/.{}", roots_dir, d_str);
                let workdir = format!("{}/.w_{}", roots_dir, d_str);
                fs::create_dir_all(&upperdir).unwrap();
                fs::create_dir_all(&workdir).unwrap();

                Command::new("mount")
                    .arg("-t")
                    .arg("overlay")
                    .arg("overlay")
                    .arg("-o")
                    .arg(format!("{}/{}", recovery_dir, d_str))
                    .spawn()
                    .expect("unable to mount overlay");
            }
        }
    }
}

fn foxmount(roots: PathBuf) {
    println!("foxmount: Checking for recovery");
    let cmdline = std::fs::read_to_string("/proc/cmdline")
        .expect("unable to read /proc/cmdline");
    match cmdline.contains("recovery=true") {
        true => recovery(),
        false => {}
    }
    println!("foxmount: Checking for config");
    if Path::new("/sysroot/roots/foxmount.sh").exists() {
        println!("foxmount: Running config");
        Command::new("source")
            .arg("/sysroot/roots/foxmount.sh")
            .spawn().expect("failed to run foxmount.sh");
    }
    let mut s = System::new();
    s.refresh_disks_list();
    let mut overlay = fs::canonicalize("/dev/disk/by-label/OVERLAY");
    match overlay {
        Ok(_) => {}
        Err(e) => {
            println!("foxmount: FATAL: could not find overlays! --> {e}");
            recovery();
            exit(1);
        }
    }
    let overlay = overlay.unwrap();
    for disk in s.disks() {
        if disk.mount_point() == overlay {
            if disk.file_system() != b"" {
                println!("foxmount: Overlays on label, traditional FS layout");
                println!("foxmount: Mounting overlay location");
                let overlay_res = Mount::builder()
                    .mount(&overlay,"/sysroot/overlay");
                println!("foxmount: Mounting home");
                let home = fs::canonicalize("/dev/disk/by-label/HOME");
                match home {
                    Ok(home) => {
                        let home_res = Mount::builder()
                            .mount(&home, "/sysroot/home");
                    },
                    Err(e) => {
                        println!("foxmount: FATAL: could not find home! {e}");
                        recovery();
                        exit(1);
                    }
                }

            }
        }else if disk.mount_point() == roots {
            if disk.file_system() == b"btrfs" {
                println!("foxmount: btrfs found, remounting with compression");
                let roots_res = Mount::builder()
                    .flags(MountFlags::REMOUNT)
                    .data("compress=zstd")
                    .mount(&roots, "/sysroot/roots");
                println!("foxmount: Mounting overlay subvolume");
                let overlay_res = Mount::builder()
                    .data("subvol=home")
                    .mount(&roots, "/sysroot/overlay");
                println!("foxmount: Mounting home subvolume");
                let overlay_res = Mount::builder()
                    .data("subvol=home")
                    .mount(&roots, "/sysroot/home");
                println!("foxmount: Setting overlay paths");
                let etc_path= "/sysroot/overlay/etc";
                let var_path= "/sysroot/overlay/var";
                let usr_path= "/sysroot/overlay/usr";

            }
        }else if disk.mount_point() ==
    }

}

fn main() {
    let overlay_path= "/sysroot/overlay";
    println!("--- foxmount ---");

    println!("foxmount: Getting roots");
    let mut roots = fs::canonicalize("/dev/disk/by-label/ROOTS");
    match roots {
        Ok(ref roots) => {
            println!("foxmount: Got root --> {:?}", roots);
            println!("foxmount: Mounting roots");
            let roots_res = Mount::builder()
                .mount(&roots, "/sysroot/roots");
        },
        Err(e) => {
            println!("foxmount: FATAL: can not find ROOTS --> {e}");
            exit(1);
        }
    };
    let etc_path= "/sysroot/overlay";
    let var_path= "/sysroot/overlay";
    let usr_path= "/sysroot/overlay";


    foxmount(roots.unwrap())
}
