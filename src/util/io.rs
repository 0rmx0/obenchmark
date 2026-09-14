//! Primitives d'E/S pour benchmarks disque crédibles.
//!
//! Contournement du cache de page du noyau lorsque c'est possible :
//! - Linux : `O_DIRECT` (E/S directs, tampons alignés).
//! - Windows : `FILE_FLAG_NO_BUFFERING` + `FILE_FLAG_WRITE_THROUGH`.
//! - macOS : repli sur E/S tamponnées (pas d'équivalent `O_DIRECT` sans libc).
//!
//! Quand le mode direct n'est pas disponible (ex. tmpfs, certains filesystems),
//! on replie sur des E/S tamponnées : le résultat reflète alors le support
//! sous-jacent (souvent de la RAM, donc non représentatif d'un vrai disque).

use std::alloc::Layout;
use std::fs::{File, OpenOptions};
use std::path::Path;

/// Alignement requis pour les E/S directs (couvre secteurs 512 o et 4 Kio).
pub const DIRECT_ALIGN: usize = 4096;

/// Tampon mémoire aligné utilisé pour les E/S directs.
pub struct AlignedBuffer {
    ptr: *mut u8,
    layout: Layout,
    len: usize,
}

// Le pointeur est propriétaire et non partagé entre threads via cette struct.
unsafe impl Send for AlignedBuffer {}

impl AlignedBuffer {
    /// Alloue un tampon de `len` octets, aligné sur `DIRECT_ALIGN`, rempli de zéros.
    pub fn zeroed(len: usize) -> Self {
        assert!(
            len > 0 && len.is_multiple_of(DIRECT_ALIGN),
            "taille non alignée"
        );
        let layout = Layout::from_size_align(len, DIRECT_ALIGN).expect("layout invalide");
        // SAFETY : `alloc_zeroed` retourne un pointeur valide ou nul ; on vérifie.
        let ptr = unsafe { std::alloc::alloc_zeroed(layout) };
        assert!(!ptr.is_null(), "échec d'allocation alignée");
        Self { ptr, layout, len }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY : `ptr` est valide pour `len` octets pendant la durée de vie de `Self`.
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    pub fn as_slice(&self) -> &[u8] {
        // SAFETY : idem, en lecture seule.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.len
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        // SAFETY : `ptr` et `layout` proviennent de `alloc_zeroed` et sont inchangés.
        unsafe { std::alloc::dealloc(self.ptr, self.layout) };
    }
}

#[cfg(target_os = "linux")]
const O_DIRECT: i32 = 0o40000;

#[cfg(target_os = "windows")]
const FILE_FLAG_NO_BUFFERING: u32 = 0x2000_0000;
#[cfg(target_os = "windows")]
const FILE_FLAG_WRITE_THROUGH: u32 = 0x8000_0000;

/// Ouvre un fichier en lecture en essayant d'abord les E/S directs.
/// Renvoie `(fichier, est_direct)` pour savoir si le contournement du cache est actif.
pub fn open_read(path: &Path) -> std::io::Result<(File, bool)> {
    if let Ok(f) = try_open_direct_read(path) {
        return Ok((f, true));
    }
    Ok((OpenOptions::new().read(true).open(path)?, false))
}

/// Ouvre un fichier en écriture en essayant d'abord les E/S directs.
pub fn open_write(path: &Path) -> std::io::Result<(File, bool)> {
    if let Ok(f) = try_open_direct_write(path) {
        return Ok((f, true));
    }
    Ok((
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)?,
        false,
    ))
}

/// Ouvre un fichier en lecture+écriture en essayant d'abord les E/S directs.
pub fn open_rw(path: &Path) -> std::io::Result<(File, bool)> {
    if let Ok(f) = try_open_direct_rw(path) {
        return Ok((f, true));
    }
    Ok((OpenOptions::new().read(true).write(true).open(path)?, false))
}

#[cfg(target_os = "linux")]
fn try_open_direct_read(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .custom_flags(O_DIRECT)
        .open(path)
}

#[cfg(target_os = "linux")]
fn try_open_direct_write(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .custom_flags(O_DIRECT)
        .open(path)
}

#[cfg(target_os = "linux")]
fn try_open_direct_rw(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(O_DIRECT)
        .open(path)
}

#[cfg(target_os = "windows")]
fn try_open_direct_read(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .attributes(FILE_FLAG_NO_BUFFERING)
        .open(path)
}

#[cfg(target_os = "windows")]
fn try_open_direct_write(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .attributes(FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH)
        .open(path)
}

#[cfg(target_os = "windows")]
fn try_open_direct_rw(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;
    OpenOptions::new()
        .read(true)
        .write(true)
        .attributes(FILE_FLAG_NO_BUFFERING | FILE_FLAG_WRITE_THROUGH)
        .open(path)
}

// macOS : pas d'O_DIRECT en std ; repli tamponné.
#[cfg(target_os = "macos")]
fn try_open_direct_read(path: &Path) -> std::io::Result<File> {
    Err(std::io::Error::from_raw_os_error(libc_like_enotsup()))
}
#[cfg(target_os = "macos")]
fn try_open_direct_write(path: &Path) -> std::io::Result<File> {
    Err(std::io::Error::from_raw_os_error(libc_like_enotsup()))
}
#[cfg(target_os = "macos")]
fn try_open_direct_rw(path: &Path) -> std::io::Result<File> {
    Err(std::io::Error::from_raw_os_error(libc_like_enotsup()))
}
#[cfg(target_os = "macos")]
fn libc_like_enotsup() -> i32 {
    95 // ENOTSUP sur macOS
}

/// Lecture positionnelle atomique (`pread` / `seek_read`).
pub fn pread(file: &File, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.read_at(buf, offset)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        file.seek_read(buf, offset)
    }
}

/// Écriture positionnelle atomique (`pwrite` / `seek_write`).
pub fn pwrite(file: &File, buf: &[u8], offset: u64) -> std::io::Result<usize> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.write_at(buf, offset)
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        file.seek_write(buf, offset)
    }
}

/// Aligne un offset vers le bas sur `block` (utile pour les E/S directs).
pub fn align_down(offset: u64, block: u64) -> u64 {
    offset & !(block - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligned_buffer_is_aligned_and_zeroed() {
        let mut b = AlignedBuffer::zeroed(4096);
        assert_eq!(b.len(), 4096);
        assert_eq!(b.as_slice().as_ptr() as usize % DIRECT_ALIGN, 0);
        assert!(b.as_slice().iter().all(|&x| x == 0));
        b.as_mut_slice()[0] = 1;
        assert_eq!(b.as_slice()[0], 1);
    }

    #[test]
    fn align_down_works() {
        assert_eq!(align_down(5000, 4096), 4096);
        assert_eq!(align_down(4096, 4096), 4096);
        assert_eq!(align_down(0, 4096), 0);
    }

    #[test]
    fn open_read_returns_handle() {
        let dir = std::env::temp_dir();
        let p = dir.join("obench_io_test.bin");
        let _ = std::fs::remove_file(&p);
        std::fs::write(&p, b"data").unwrap();
        let (f, _direct) = open_read(&p).unwrap();
        drop(f);
        let _ = std::fs::remove_file(&p);
    }
}
