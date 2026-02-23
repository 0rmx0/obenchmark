//! Benchmark disque : écriture puis lecture d’un fichier temporaire.

use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

pub fn disk_test() -> u64 {
    // Crée un fichier temporaire dans le répertoire courant.
    let mut path = std::env::temp_dir();
    path.push("lumo_benchmark_tmp.bin");

    // Taille du fichier : 100 Mo
    let size: usize = 100 * 1024 * 1024;
    let data = vec![0u8; size];

    // Écriture
    let start = std::time::Instant::now();
    {
        let mut f = File::create(&path).expect("Impossible de créer le fichier");
        f.write_all(&data).expect("Échec d’écriture");
        f.sync_all().expect("Sync échoué");
    }
    let write_dur = start.elapsed().as_secs_f64();

    // Lecture
    let start = std::time::Instant::now();
    {
        let mut f = OpenOptions::new()
            .read(true)
            .open(&path)
            .expect("Impossible d’ouvrir le fichier");
        let mut buf = vec![0u8; size];
        f.read_exact(&mut buf).expect("Échec de lecture");
    }
    let read_dur = start.elapsed().as_secs_f64();

    // Nettoyage
    let _ = std::fs::remove_file(&path);

    // Score similaire au benchmark RAM
    ((1_000_000.0) / (write_dur + read_dur)) as u64
}
