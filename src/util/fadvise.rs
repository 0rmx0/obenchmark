use std::fs::File;

/// Ask the kernel to drop the page cache for this file (Linux only).
/// Lets disk read benchmarks measure the device instead of RAM.
#[cfg(target_os = "linux")]
pub fn drop_page_cache(file: &File) {
    use std::os::unix::io::AsRawFd;

    extern "C" {
        fn posix_fadvise(fd: i32, offset: i64, len: i64, advice: i32) -> i32;
    }
    const POSIX_FADV_DONTNEED: i32 = 4;

    unsafe {
        posix_fadvise(file.as_raw_fd(), 0, 0, POSIX_FADV_DONTNEED);
    }
}

#[cfg(not(target_os = "linux"))]
pub fn drop_page_cache(_file: &File) {}
