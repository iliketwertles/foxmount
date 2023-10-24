use std::fs;
use std::path::{Path, PathBuf};
use sysinfo::{DiskExt, System, SystemExt};
use std::process::{Command, exit}; //:sob:
use sys_mount::{
    Mount,
    MountFlags,
    UnmountFlags
};

// test comment for commit

fn foxsnapshot_revert() {
    //this is to reduce repetition, can probably be done away with in the future
    println!("foxmount: Checking for foxsnapshot revert");
    if Path::new("/sysroot/roots/.revert").exists() {
        Command::new("btrfs")
            .arg("subvolume")
            .arg("delete")
            .arg("/sysroot/overlay/usr")
            .spawn()
            .expect("foxmount: Failed to delete subvolume");


        fs::remove_file("/sysroot/roots/.revert").expect("foxmount: Failed to remove .revert");
    }
}

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
    Mount::builder()
        .fstype("tmpfs")
        .mount("tmpfs", "/sysroot/roots/.recovery")
        .expect("foxmount: Failed to mount .recovery");
    std::env::set_current_dir("/sysroot/roots/.recovery")
        .expect("how.");
    if let Ok(entries) = fs::read_dir(recovery_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let d = entry.file_name();

                //let lowerdir = format!("{}/{}:{}/{}", recovery_dir, d.to_string_lossy(), recovery_dir, d.to_string_lossy());
                let upperdir = format!("{}/.{}", roots_dir, d.to_string_lossy());
                let workdir = format!("{}/.w_{}", roots_dir, d.to_string_lossy());
                fs::create_dir_all(&upperdir).unwrap();
                fs::create_dir_all(&workdir).unwrap();

                Mount::builder()
                    .fstype("overlay")
                    .data(format!("lowerdir=/sysroot/.recovery/{}:/sysroot/{},upperdir=/sysroot/roots/.recovery/.{},workdir=/sysroot/roots/.recovery/.w_{}", d.to_string_lossy(), d.to_string_lossy(), d.to_string_lossy(), d.to_string_lossy()).as_str())
                    .mount("overlay", "/sysroot/")
                    .expect("foxmount: im not shocked");
                /*Command::new("mount")
                    .arg("-t")
                    .arg("overlay")
                    .arg("overlay")
                    .arg("-o")
                    .arg(format!("{}/{}", recovery_dir, d.to_string()))
                    .spawn()
                    .expect("unable to mount overlay");*/
            }
        }
    }
}

fn foxmount(roots: PathBuf, xenia: PathBuf) {
    let mut checkvar = 0;
    let mut overlay_path = 0;
    println!("foxmount: Checking for recovery");
    let cmdline = fs::read_to_string("/proc/cmdline")
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
    let overlay = fs::canonicalize("/dev/disk/by-label/OVERLAY");
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
        if disk.name() == overlay.as_os_str() {
            if disk.file_system() != b"" {
                println!("foxmount: Overlays on label, traditional FS layout");
                println!("foxmount: Mounting overlay location");
                Mount::builder()
                    .mount(&overlay,"/sysroot/overlay")
                    .expect("foxmount: Failed to mount overlay");
                println!("foxmount: Mounting home");
                let home = fs::canonicalize("/dev/disk/by-label/HOME");
                match home {
                    Ok(home) => {
                        Mount::builder()
                            .mount(&home, "/sysroot/home")
                            .expect("foxmount: Failed to mount home");
                        checkvar += 1;
                    },
                    Err(e) => {
                        println!("foxmount: FATAL: could not find home! {e}");
                        recovery();
                        exit(1);
                    }
                }
            }
        }else if disk.name() == roots.as_os_str() {
            if disk.file_system() == b"btrfs" {
                println!("foxmount: btrfs found, remounting with compression");
                Mount::builder()
                    .flags(MountFlags::REMOUNT)
                    .data("compress=zstd")
                    .mount(&roots, "/sysroot/roots")
                    .expect("foxmount: Failed to mount roots");
                println!("foxmount: Mounting overlay subvolume");
                Mount::builder()
                    .data("subvol=home")
                    .mount(&roots, "/sysroot/overlay")
                    .expect("foxmount: Failed to mount overlay");
                println!("foxmount: Mounting home subvolume");
                Mount::builder()
                    .data("subvol=home")
                    .mount(&roots, "/sysroot/home")
                    .expect("foxmount: Failed to mount home");
                println!("foxmount: Setting overlay paths");
                //let etc_path= "/sysroot/overlay/etc";
                //let var_path= "/sysroot/overlay/var";
                //let usr_path= "/sysroot/overlay/usr";
                overlay_path += 1;
                checkvar += 1;

            }
        }else if disk.name() == xenia.as_os_str() && disk.file_system() == b"crypto_LUKS" {
                println!("foxmount: LUKS found");
                Command::new("plymouth")
                    .arg("ask-for-password")
                    .arg("--command='cryptsetup luksOpen /dev/disk/by-label/XENIA xenia'")
                    .arg("--prompt='Enter decryption key'")
                    .spawn()
                    .expect("Failure running plymouth");
                println!("foxmount: Mounting overlay subvolume");
                Mount::builder()
                    .data("subvol=overlay,compress=zstd")
                    .mount("/dev/mapper/xenia", "/sysroot/overlay")
                    .expect("foxmount: Failed to mount overlay");

                println!("foxmount: Mounting home");
                Mount::builder()
                    .data("subvol=home")
                    .mount("/dev/mapper/xenia", "/sysroot/home")
                    .expect("foxmount: Failed to mount home");

                println!("foxmount: Setting overlay paths");
                //let etc_path= "/sysroot/overlay/etc";
                //let var_path= "/sysroot/overlay/var";
                //let usr_path= "/sysroot/overlay/usr";
                overlay_path += 2;
                checkvar += 1;
        }
    }
    if checkvar > 0 {
        println!("foxmount: Something good happened (probably)");
    } else {
        println!("foxmount: Something good didnt happen (probably)");
        recovery();
        exit(1);
    }

    println!("foxmount: Creating overlay and overlay work directories if they don't exist");
    match overlay_path {
        0 => {
            fs::create_dir_all("/sysroot/overlay/etc").unwrap();
            fs::create_dir_all("/sysroot/overlay/var").unwrap();
            fs::create_dir_all("/sysroot/overlay/usr").unwrap();
            fs::create_dir_all("/sysroot/overlay/etcw").unwrap();
            fs::create_dir_all("/sysroot/overlay/varw").unwrap();
            fs::create_dir_all("/sysroot/overlay/usrw").unwrap();
            foxsnapshot_revert();
            Mount::builder()
                .fstype("overlay")
                .data("lowerdir=/sysroot/usr,upperdir=/sysroot/overlay/usr,workdir=/sysroot/overlay/usrw")
                .flags(MountFlags::RDONLY)
                .mount(&overlay, "/sysroot/usr")
                .expect("foxmount: Failed to mount usr overlay");
            Mount::builder()
                .fstype("overlay")
                .data("lowerdir=/sysroot/etc,upperdir=/sysroot/overlay/etc,workdir=/sysroot/overlay/etcw")
                .mount(&overlay, "/sysroot/etc")
                .expect("foxmount: Failed to mount etc overlay");
            Mount::builder()
                .fstype("overlay")
                .data("lowerdir=/sysroot/var,upperdir=/sysroot/overlay/var,workdir=/sysroot/overlay/varw")
                .mount(&overlay, "/sysroot/var")
                .expect("foxmount: Failed to mount var overlay");
        },
        1..=2 => {
            fs::create_dir_all("/sysroot/overlay/etc/etc").unwrap();
            fs::create_dir_all("/sysroot/overlay/var/var").unwrap();
            fs::create_dir_all("/sysroot/overlay/usr/usr").unwrap();
            fs::create_dir_all("/sysroot/overlay/etc/etcw").unwrap();
            fs::create_dir_all("/sysroot/overlay/var/varw").unwrap();
            fs::create_dir_all("/sysroot/overlay/usr/usrw").unwrap();
            foxsnapshot_revert();
            Mount::builder()
                .fstype("overlay")
                .data("lowerdir=/sysroot/usr,upperdir=/sysroot/overlay/usr/usr,workdir=/sysroot/overlay/usr/usrw")
                .flags(MountFlags::RDONLY)
                .mount(&overlay, "/sysroot/usr")
                .expect("foxmount: Failed to mount usr overlay");
            Mount::builder()
                .fstype("overlay")
                .data("lowerdir=/sysroot/etc,upperdir=/sysroot/overlay/etc/etc,workdir=/sysroot/overlay/etc/etcw")
                .mount(&overlay, "/sysroot/etc")
                .expect("foxmount: Failed to mount etc overlay");
            Mount::builder()
                .fstype("overlay")
                .data("lowerdir=/sysroot/var,upperdir=/sysroot/overlay/var/var,workdir=/sysroot/overlay/var/varw")
                .mount(&overlay, "/sysroot/var")
                .expect("foxmount: Failed to mount var overlay");
        },
        _ => panic!("how even?")
    }
    println!("foxmount: Finished mounting overlays");
}

fn main() {
    println!("--- foxmount ---");

    println!("foxmount: Getting roots");
    let roots = fs::canonicalize("/dev/disk/by-label/ROOTS");
    match roots {
        Ok(ref roots) => {
            println!("foxmount: Got root --> {:?}", roots);
            println!("foxmount: Mounting roots");
            Mount::builder()
                .mount(roots, "/sysroot/roots")
                .expect("foxmount: failed to mount roots (big panik)");
        },
        Err(_) => panic!("foxmount: FATAL: can not find ROOTS")
    };
    let xenia = fs::canonicalize("/dev/disk/by-label/XENIA");

    foxmount(roots.unwrap(), xenia.unwrap_or_default())
}
