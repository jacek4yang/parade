//! Minimal raw Linux syscall support — **no libc, no C runtime**.
//!
//! The agent reaches the kernel directly through the syscall ABI (inline
//! assembly), so its metric-collection path carries zero C-library
//! dependency and speaks to the raw kernel interface on every architecture
//! we target.
//!
//! Almost everything the agent needs is exposed by the kernel through the
//! `/proc` and `/sys` pseudo-filesystems (plain file reads). The single
//! exception is **filesystem capacity**, which has no procfs interface — so
//! we issue `statfs(2)` ourselves.
//!
//! Architecture coverage: raw `statfs(2)` is implemented for the 64-bit
//! targets that make up the entire practical VPS population — x86_64,
//! aarch64, riscv64 — all of which share the LP64 `asm-generic` `statfs`
//! layout and the same register-based calling convention (only the syscall
//! number and trap instruction differ). On any other architecture the agent
//! still builds and reports every other metric; disk capacity degrades to 0
//! rather than risking an incorrect struct decode.

#![allow(non_upper_case_globals)]

// --------------------------------------------------------------- raw trap

#[cfg(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "riscv64"
))]
mod raw {
    use core::arch::asm;

    // asm-generic syscall table: statfs = 43 (aarch64, riscv64).
    // x86_64 has its own table where statfs = 137.
    #[cfg(target_arch = "x86_64")]
    pub const SYS_STATFS: usize = 137;
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
    pub const SYS_STATFS: usize = 43;

    /// Two-argument syscall. Returns the kernel's signed result (negative =
    /// -errno). `unsafe`: caller guarantees the pointers are valid.
    #[cfg(target_arch = "x86_64")]
    #[inline]
    pub unsafe fn syscall2(n: usize, a1: usize, a2: usize) -> isize {
        let ret: isize;
        asm!(
            "syscall",
            inlateout("rax") n => ret,
            in("rdi") a1,
            in("rsi") a2,
            lateout("rcx") _,   // clobbered by syscall
            lateout("r11") _,   // clobbered by syscall
            options(nostack),
        );
        ret
    }

    #[cfg(target_arch = "aarch64")]
    #[inline]
    pub unsafe fn syscall2(n: usize, a1: usize, a2: usize) -> isize {
        let ret: isize;
        asm!(
            "svc #0",
            in("x8") n,
            inlateout("x0") a1 => ret,
            in("x1") a2,
            options(nostack),
        );
        ret
    }

    #[cfg(target_arch = "riscv64")]
    #[inline]
    pub unsafe fn syscall2(n: usize, a1: usize, a2: usize) -> isize {
        let ret: isize;
        asm!(
            "ecall",
            in("a7") n,
            inlateout("a0") a1 => ret,
            in("a1") a2,
            options(nostack),
        );
        ret
    }
}

// ------------------------------------------------------------- statfs(2)

/// LP64 `asm-generic` `struct statfs` (x86_64 / aarch64 / riscv64 share it).
/// We only read `f_bsize`, `f_blocks`, `f_bfree`, but map the whole struct so
/// the kernel writes into correctly-sized storage.
#[repr(C)]
#[derive(Default)]
struct Statfs {
    f_type: i64,
    f_bsize: i64,
    f_blocks: u64,
    f_bfree: u64,
    f_bavail: u64,
    f_files: u64,
    f_ffree: u64,
    f_fsid: [i32; 2],
    f_namelen: i64,
    f_frsize: i64,
    f_flags: i64,
    f_spare: [i64; 4],
}

pub struct FilesystemCapacity {
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
}

/// Filesystem byte and inode capacity at `path`.
///
/// `free` is the kernel's `f_bfree` (all free blocks), matching how `df`
/// computes total/used. Returns `None` if the syscall fails or the target
/// architecture has no raw implementation.
#[cfg(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "riscv64"
))]
pub fn statfs_capacity(path: &str) -> Option<FilesystemCapacity> {
    // NUL-terminate without libc's CString (avoid the dependency entirely).
    let mut buf = Vec::with_capacity(path.len() + 1);
    buf.extend_from_slice(path.as_bytes());
    if buf.contains(&0) {
        return None; // embedded NUL — not a real path
    }
    buf.push(0);

    let mut st = Statfs::default();
    let ret = unsafe {
        raw::syscall2(
            raw::SYS_STATFS,
            buf.as_ptr() as usize,
            &mut st as *mut Statfs as usize,
        )
    };
    if ret < 0 {
        return None;
    }
    // Block size: prefer fragment size when the kernel reports it (matches
    // statvfs/df); otherwise the optimal transfer block size.
    let bs = if st.f_frsize > 0 {
        st.f_frsize as u64
    } else {
        st.f_bsize.max(0) as u64
    };
    let total = st.f_blocks.saturating_mul(bs);
    let free = st.f_bfree.saturating_mul(bs);
    Some(FilesystemCapacity {
        total_bytes: total,
        free_bytes: free,
        total_inodes: st.f_files,
        free_inodes: st.f_ffree,
    })
}

#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "riscv64"
)))]
pub fn statfs_capacity(_path: &str) -> Option<FilesystemCapacity> {
    // No raw statfs for this architecture — disk capacity reported as 0.
    None
}
