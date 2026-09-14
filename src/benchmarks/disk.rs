use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::engines::benchmark::Benchmark;
use crate::util::io::{align_down, open_read, open_rw, open_write, pread, pwrite, AlignedBuffer};

/// Taille des fichiers utilisés pour les tests.
const SEQ_FILE_SIZE_BYTES: u64 = 512 * 1024 * 1024; // 512 MiB
const RAND_FILE_SIZE_BYTES: u64 = 1024 * 1024 * 1024; // 1 GiB

/// Tailles de blocs.
const SEQ_BLOCK_SIZE: usize = 1024 * 1024; // 1 MiB
const RAND_32K_BLOCK_SIZE: usize = 32 * 1024;
const RAND_4K_BLOCK_SIZE: usize = 4 * 1024;

/// Durée des tests random (proche CrystalDiskMark).
const RANDOM_TEST_DURATION_SECS: u64 = 5;

/// Profondeurs de file approximatives (nb de threads).
const QD_32K: usize = 20; // "QD20"
const QD_4K: usize = 1; // "QD1"

/// Chauffe + passes répétées pour les mesures séquentielles.
const SEQ_WARMUP: usize = 1;
const SEQ_PASSES: usize = 3;

fn temp_file(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("obenchmark_{}.bin", name))
}

/// Fichier temporaire de benchmark supprimé automatiquement à la libération,
/// même en cas d'erreur ou de panique pendant le test.
struct TestFile {
    path: PathBuf,
}

impl TestFile {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Crée un fichier de taille `target_size` rempli de zéros, via E/S alignées
/// lorsque le mode direct est supporté (ainsi le contenu est bien sur disque).
/// Réduit la taille par 2 en cas d'erreur pour gérer le manque d'espace disque.
fn create_test_file_best_effort(name: &str, target_size: u64) -> Result<(TestFile, u64)> {
    const MIN_SIZE: u64 = 64 * 1024 * 1024; // 64 MiB minimum

    let mut size = target_size;
    let mut last_err: Option<anyhow::Error> = None;
    let path = temp_file(name);

    while size >= MIN_SIZE {
        match create_file_with_size(&path, size) {
            Ok(_) => return Ok((TestFile { path }, size)),
            Err(e) => {
                last_err = Some(e);
                size /= 2;
            }
        }
    }

    // Nettoyage éventuel d'un fichier partiellement créé.
    let _ = std::fs::remove_file(&path);
    Err(last_err.unwrap_or_else(|| anyhow!("Espace disque insuffisant pour le benchmark disque")))
}

fn create_file_with_size(path: &Path, size: u64) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }

    // Pré-allocation de la taille finale ( évite la fragmentation ).
    {
        let file = std::fs::File::create(path)?;
        file.set_len(size)?;
    }

    // Remplissage du fichier avec des données pseudo-aléatoires via E/S directs
    // si possible, pour s'assurer que le contenu n'est pas une sparse region
    // (qui serait lue depuis zéro sans toucher le disque).
    fill_file(path, size)?;
    Ok(())
}

fn fill_file(path: &Path, size: u64) -> Result<()> {
    let (file, _direct) = open_write(path).or_else(|_| {
        // Repli tamponné si l'ouverture directe échoue.
        std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(path)
            .map(|f| (f, false))
    })?;

    let block = SEQ_BLOCK_SIZE;
    let mut buf = AlignedBuffer::zeroed(block);
    // Remplissage pseudo-aléatoire déterministe pour éviter les sparse regions.
    // Graine hex pure (les lettres non-hex casseraient le lexage du littéral).
    let mut rng = StdRng::seed_from_u64(0x0DEADBEEF00DCAFE);
    for b in buf.as_mut_slice() {
        *b = rng.gen();
    }

    let mut offset: u64 = 0;
    while offset < size {
        let to_write = std::cmp::min(block as u64, size - offset) as usize;
        let n = pwrite(&file, &buf.as_slice()[..to_write], offset)?;
        if n == 0 {
            return Err(anyhow!("écriture nulle pendant le remplissage"));
        }
        offset += n as u64;
    }
    file.sync_all()?;
    Ok(())
}

fn bytes_per_sec_to_mb_per_sec(bytes: u64, secs: f64) -> u64 {
    if secs <= 0.0 {
        return 0;
    }
    let mb = (bytes as f64) / (1024.0 * 1024.0);
    (mb / secs) as u64
}

/// Sens d'E/S aléatoire.
#[derive(Clone, Copy)]
enum IoDir {
    Read,
    Write,
}

/// IOPS aléatoires via E/S positionnels (pread/pwrite), contournant le cache OS.
/// Chaque thread garde son propre fichier pour éviter la contention du descripteur.
fn random_iops(
    path: &Path,
    file_size: u64,
    block_size: usize,
    queue_depth: usize,
    dir: IoDir,
) -> Result<u64> {
    let duration = Duration::from_secs(RANDOM_TEST_DURATION_SECS);
    let max_offset = align_down(
        file_size
            .checked_sub(block_size as u64)
            .ok_or_else(|| anyhow!("Fichier de test trop petit pour la taille de bloc"))?,
        block_size as u64,
    );

    let mut handles = Vec::with_capacity(queue_depth);
    for thread_id in 0..queue_depth {
        let path = path.to_owned();
        let handle = thread::spawn(move || -> Result<u64> {
            let mut rng = StdRng::seed_from_u64(0xD15C_0000 + thread_id as u64);
            // Un descripteur par thread : évite les seeks concurrents.
            let (file, _direct) = open_rw(&path)?;
            let mut buf = AlignedBuffer::zeroed(block_size);
            // Chauffe : on ignore quelques opérations pour stabiliser.
            for _ in 0..16 {
                let off = align_down(rng.gen_range(0..=max_offset), block_size as u64);
                match dir {
                    IoDir::Read => {
                        pread(&file, buf.as_mut_slice(), off)?;
                    }
                    IoDir::Write => {
                        pwrite(&file, buf.as_slice(), off)?;
                    }
                }
            }

            let start = Instant::now();
            let mut ops: u64 = 0;
            while start.elapsed() < duration {
                let offset = align_down(rng.gen_range(0..=max_offset), block_size as u64);
                match dir {
                    IoDir::Read => {
                        pread(&file, buf.as_mut_slice(), offset)?;
                    }
                    IoDir::Write => {
                        pwrite(&file, buf.as_slice(), offset)?;
                    }
                }
                ops += 1;
            }
            Ok(ops)
        });
        handles.push(handle);
    }

    let mut total_ops: u64 = 0;
    for h in handles {
        total_ops += h.join().unwrap_or_else(|_| Ok(0))?;
    }
    let secs = duration.as_secs().max(1);
    Ok(total_ops / secs)
}

/// Lecture séquentielle gros blocs (MB/s), en contournant le cache OS.
pub struct DiskSequentialRead;

impl Benchmark for DiskSequentialRead {
    fn name(&self) -> &str {
        "Disk Seq Read"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        let (test_file, size) = create_test_file_best_effort("seq_read", SEQ_FILE_SIZE_BYTES)?;
        let path = test_file.path();

        let mut samples: Vec<u64> = Vec::with_capacity(SEQ_PASSES);
        for pass in 0..(SEQ_WARMUP + SEQ_PASSES) {
            let (file, _direct) = open_read(path)?;
            let mut buf = AlignedBuffer::zeroed(SEQ_BLOCK_SIZE);
            let start = Instant::now();
            let mut offset: u64 = 0;
            let mut read_bytes: u64 = 0;
            while offset < size {
                let n = pread(&file, buf.as_mut_slice(), offset)?;
                if n == 0 {
                    break;
                }
                read_bytes += n as u64;
                offset += n as u64;
            }
            let elapsed = start.elapsed().as_secs_f64().max(1e-9);
            let mbps = bytes_per_sec_to_mb_per_sec(std::cmp::min(read_bytes, size), elapsed);
            if pass >= SEQ_WARMUP {
                samples.push(mbps);
            }
        }
        // Le fichier est supprimé automatiquement par le `Drop` de `test_file`.
        Ok(median_u64(&samples))
    }
}

/// Écriture séquentielle gros blocs (MB/s), en contournant le cache OS.
pub struct DiskSequentialWrite;

impl Benchmark for DiskSequentialWrite {
    fn name(&self) -> &str {
        "Disk Seq Write"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        let mut samples: Vec<u64> = Vec::with_capacity(SEQ_PASSES);
        for pass in 0..(SEQ_WARMUP + SEQ_PASSES) {
            let test_file = TestFile {
                path: temp_file("seq_write"),
            };
            let path = test_file.path();
            let (file, _direct) = open_write(path)?;
            let mut buf = AlignedBuffer::zeroed(SEQ_BLOCK_SIZE);
            // Données pseudo-aléatoires pour éviter la compression transparente.
            let mut rng = StdRng::seed_from_u64(0x5ECA + pass as u64);
            for b in buf.as_mut_slice() {
                *b = rng.gen();
            }

            let start = Instant::now();
            let mut offset: u64 = 0;
            while offset < SEQ_FILE_SIZE_BYTES {
                let to_write =
                    std::cmp::min(SEQ_BLOCK_SIZE as u64, SEQ_FILE_SIZE_BYTES - offset) as usize;
                let n = pwrite(&file, &buf.as_slice()[..to_write], offset)?;
                if n == 0 {
                    break;
                }
                offset += n as u64;
            }
            file.sync_all()?;
            let elapsed = start.elapsed().as_secs_f64().max(1e-9);
            let mbps = bytes_per_sec_to_mb_per_sec(SEQ_FILE_SIZE_BYTES, elapsed);
            if pass >= SEQ_WARMUP {
                samples.push(mbps);
            }
        }
        Ok(median_u64(&samples))
    }
}

/// IOPS en lecture aléatoire 32K avec profondeur de file ~20.
pub struct DiskRandomIOPS32K;

impl Benchmark for DiskRandomIOPS32K {
    fn name(&self) -> &str {
        "Disk IOPS 32K QD20"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        let (test_file, size) = create_test_file_best_effort("rand_32k", RAND_FILE_SIZE_BYTES)?;
        let iops = random_iops(
            test_file.path(),
            size,
            RAND_32K_BLOCK_SIZE,
            QD_32K,
            IoDir::Read,
        )?;
        Ok(iops)
    }
}

/// IOPS en écriture aléatoire 4K queue depth 1 (latence).
pub struct DiskRandomIOPS4K;

impl Benchmark for DiskRandomIOPS4K {
    fn name(&self) -> &str {
        "Disk IOPS 4K QD1"
    }

    fn weight(&self) -> u64 {
        2
    }

    fn run(&self) -> Result<u64> {
        let (test_file, size) = create_test_file_best_effort("rand_4k", RAND_FILE_SIZE_BYTES)?;
        let iops = random_iops(
            test_file.path(),
            size,
            RAND_4K_BLOCK_SIZE,
            QD_4K,
            IoDir::Write,
        )?;
        Ok(iops)
    }
}

fn median_u64(values: &[u64]) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let mut v = values.to_vec();
    v.sort_unstable();
    let mid = v.len() / 2;
    if v.len().is_multiple_of(2) {
        (v[mid - 1] + v[mid]) / 2
    } else {
        v[mid]
    }
}
