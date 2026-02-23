//! Benchmark RAM : écriture/lecture séquentielle d’un gros buffer.

pub fn memory_test() -> u64 {
    // Taille du buffer : 200 Mo
    let size: usize = 200 * 1024 * 1024;
    let mut buffer = vec![0u8; size];

    // Écriture
    let start = std::time::Instant::now();
    for i in 0..size {
        buffer[i] = (i % 256) as u8;
    }
    let write_dur = start.elapsed().as_secs_f64();

    // Lecture
    let start = std::time::Instant::now();
    let mut checksum: u64 = 0;
    for &b in &buffer {
        checksum = checksum.wrapping_add(b as u64);
    }
    let read_dur = start.elapsed().as_secs_f64();

    // Score = 1 000 000 / (write + read) (plus rapide = meilleur)
    ((1_000_000.0) / (write_dur + read_dur)) as u64
}
