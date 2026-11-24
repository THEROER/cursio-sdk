//! Mounts a two-file revision through the node on this machine and works in
//! it the way a game would: reads a file the store holds, tries one no machine
//! holds, writes a new one, then prints what changed. Needs a node with file
//! mounts (FUSE on Linux, WinFsp on Windows).
//!
//! ```sh
//! cargo run -p cursio-sdk --example revision_probe
//! ```

use std::time::{Duration, Instant};

use cursio_sdk::{App, BlobAlgo, BlobEntry, BlobHash};

fn main() {
    let cursio = App::new("revision-probe").start().expect("a valid id");
    let since = Instant::now();
    while cursio.machine().is_none() {
        assert!(since.elapsed() < Duration::from_secs(5), "no node answered");
        std::thread::sleep(Duration::from_millis(100));
    }
    let root = std::env::temp_dir().join("cursio-revision-probe");
    let _ = std::fs::remove_dir_all(&root);
    let (store, upper, state) = (root.join("store"), root.join("upper"), root.join("state"));
    std::fs::create_dir_all(&store).unwrap();

    let held = BlobHash::new(BlobAlgo::Sha256, &"a".repeat(64)).unwrap();
    let nowhere = BlobHash::new(BlobAlgo::Sha256, &"b".repeat(64)).unwrap();
    let bytes = b"hello from the store";
    std::fs::write(store.join(held.file_name()), bytes).unwrap();
    let entries = [
        BlobEntry {
            path: "mods/a.txt".into(),
            hash: held,
            size: bytes.len() as u64,
        },
        BlobEntry {
            path: "saves/w/level.dat".into(),
            hash: nowhere,
            size: 5,
        },
    ];

    let mount = cursio
        .mount_revision("probe", &entries, &store, &upper, &state)
        .expect("mounted");
    let at = mount.path().to_path_buf();
    println!("mounted at {}", at.display());
    println!("listing: {:?}", names(&at));
    println!(
        "mods/a.txt: {:?}",
        std::fs::read_to_string(at.join("mods/a.txt"))
    );
    println!(
        "saves/w/level.dat (held nowhere): {:?}",
        std::fs::read(at.join("saves/w/level.dat")).map(|b| b.len())
    );
    std::fs::create_dir_all(at.join("config")).unwrap();
    std::fs::write(at.join("config/options.txt"), "fov:90").unwrap();
    std::fs::write(at.join("mods/a.txt"), "changed here").unwrap();
    println!("changes: {:?}", cursio.revision_changes(&mount));
    println!(
        "store untouched: {:?}",
        std::fs::read_to_string(store.join(entries[0].hash.file_name()))
    );
    drop(mount);
    std::thread::sleep(Duration::from_secs(1));
    println!(
        "after unmount, {} exists: {}",
        at.display(),
        at.join("mods").exists()
    );
}

fn names(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map(|l| {
            l.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}
