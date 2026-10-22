use std::{
    alloc::{GlobalAlloc, Layout},
    cell::Cell,
    sync::atomic::{AtomicU8, AtomicU64, AtomicUsize, Ordering},
};

use serde::Serialize;

/// Allocation size classes, cheap enough to leave on: <64 KB, 64 KB-1 MB, 1-16 MB, >=16 MB.
/// A big heap made of a few huge buffers and one made of millions of small objects need
/// different fixes; this tells them apart.
static CLASS_COUNT: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
static CLASS_BYTES: [AtomicU64; 4] = [const { AtomicU64::new(0) }; 4];
/// Number of allocations of 16 MB or more, and the largest single one seen.
static HUGE_COUNT: AtomicU64 = AtomicU64::new(0);
static LARGEST: AtomicU64 = AtomicU64::new(0);
/// Set to the size of the latest allocation >= 16 MB so a logger thread can report it.
static LAST_HUGE: AtomicU64 = AtomicU64::new(0);

#[inline]
fn size_class(size: u64) -> usize {
    match size {
        0..65_536 => 0,
        65_536..1_048_576 => 1,
        1_048_576..16_777_216 => 2,
        _ => 3,
    }
}

#[inline]
fn record_size(size: u64) {
    let c = size_class(size);
    CLASS_COUNT[c].fetch_add(1, Ordering::Relaxed);
    CLASS_BYTES[c].fetch_add(size, Ordering::Relaxed);
    if c == 3 {
        HUGE_COUNT.fetch_add(1, Ordering::Relaxed);
        LARGEST.fetch_max(size, Ordering::Relaxed);
        LAST_HUGE.store(size, Ordering::Relaxed);
    }
}

/// Allocation counts and total bytes ever requested per size class, plus the number of
/// allocations of 16 MB or more and the largest single one. Totals, not live bytes.
pub fn size_class_report() -> String {
    let mb = |b: u64| b / (1024 * 1024);
    let n = |i: usize| CLASS_COUNT[i].load(Ordering::Relaxed);
    let b = |i: usize| mb(CLASS_BYTES[i].load(Ordering::Relaxed));
    format!(
        "allocs requested: <64KB {}x {}MB | 64KB-1MB {}x {}MB | 1-16MB {}x {}MB | >=16MB {}x {}MB (largest {}MB)",
        n(0), b(0), n(1), b(1), n(2), b(2), n(3), b(3),
        mb(LARGEST.load(Ordering::Relaxed))
    )
}

/// Size of the newest allocation of 16 MB or more since the last call, if any.
pub fn take_last_huge() -> Option<u64> {
    let v = LAST_HUGE.swap(0, Ordering::Relaxed);
    (v != 0).then_some(v)
}

thread_local! {
    /// This thread's net allocated bytes (allocs minus frees). Plain `Cell`: no atomics,
    /// no allocation, so it is safe to touch from inside the global allocator.
    static THREAD_NET: Cell<i64> = const { Cell::new(0) };
}

#[inline]
fn thread_net_add(delta: i64) {
    let _ = THREAD_NET.try_with(|net| net.set(net.get().wrapping_add(delta)));
}

/// Net bytes this thread has allocated and not freed itself. Memory freed by another
/// thread lowers that thread's figure, not this one's.
pub fn thread_net_bytes() -> i64 {
    THREAD_NET.try_with(Cell::get).unwrap_or(0)
}

const FALLBACK_SLOT: usize = 0;
const MAX_THREADS: usize = 256;

#[repr(C, align(64))]
struct ThreadCounters {
    alloc_count: AtomicU64,
    alloc_bytes: AtomicU64,
    dealloc_count: AtomicU64,
    dealloc_bytes: AtomicU64,
}

impl ThreadCounters {
    const fn new() -> Self {
        Self {
            alloc_count: AtomicU64::new(0),
            alloc_bytes: AtomicU64::new(0),
            dealloc_count: AtomicU64::new(0),
            dealloc_bytes: AtomicU64::new(0),
        }
    }
}

static SLOTS: [ThreadCounters; MAX_THREADS] = [const { ThreadCounters::new() }; MAX_THREADS];

static NEXT_SLOT: AtomicUsize = AtomicUsize::new(1);

static SATURATED: AtomicU64 = AtomicU64::new(0);

thread_local! {
    static SLOT: Cell<usize> = const { Cell::new(usize::MAX) };
}

fn slot() -> &'static ThreadCounters {
    let idx = SLOT
        .try_with(|cell| {
            let mut idx = cell.get();
            if idx == usize::MAX {
                idx = NEXT_SLOT.fetch_add(1, Ordering::Relaxed);
                if idx >= MAX_THREADS {
                    SATURATED.fetch_add(1, Ordering::Relaxed);
                    idx = MAX_THREADS - 1;
                    NEXT_SLOT.store(MAX_THREADS, Ordering::Relaxed);
                }
                cell.set(idx);
            }
            idx
        })
        .unwrap_or(FALLBACK_SLOT);
    &SLOTS[idx]
}

fn record_alloc(size: u64) {
    if !counting_enabled() {
        return;
    }
    let s = slot();
    s.alloc_count.fetch_add(1, Ordering::Relaxed);
    s.alloc_bytes.fetch_add(size, Ordering::Relaxed);
}

fn record_dealloc(size: u64) {
    if !counting_enabled() {
        return;
    }
    let s = slot();
    s.dealloc_count.fetch_add(1, Ordering::Relaxed);
    s.dealloc_bytes.fetch_add(size, Ordering::Relaxed);
}

/// Whether the counting allocator is actually counting. It is off unless
/// `IW4L_COUNTING_ALLOC` is set, so every figure derived from it is absent
/// rather than zero on an ordinary run — and a caller that cannot tell those
/// apart will report "no allocations" for "nobody was counting".
pub fn counting_enabled() -> bool {
    static STATE: AtomicU8 = AtomicU8::new(2);
    match STATE.load(Ordering::Relaxed) {
        0 => false,
        1 => true,
        _ => {
            let on = env_key_present(b"IW4L_COUNTING_ALLOC\0");
            STATE.store(u8::from(on), Ordering::Relaxed);
            on
        }
    }
}

fn env_key_present(key_nul: &[u8]) -> bool {
    debug_assert_eq!(key_nul.last().copied(), Some(0));
    !unsafe { getenv(key_nul.as_ptr().cast()) }.is_null()
}

unsafe extern "C" {
    fn getenv(name: *const core::ffi::c_char) -> *mut core::ffi::c_char;
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    fn malloc_trim(pad: usize) -> core::ffi::c_int;
    /// libmalloc (Apple, already linked through libSystem): returns cached free pages to
    /// the OS. `zone` NULL = every zone, `goal` 0 = as much as possible. Returns bytes freed.
    #[cfg(target_os = "ios")]
    fn malloc_zone_pressure_relief(zone: *mut core::ffi::c_void, goal: usize) -> usize;
}

/// Hands free heap pages back to the OS and reports how long it took and how many bytes
/// were released (0 where the platform gives no figure). Safe from any thread; it only
/// drops allocator caches, it never invalidates a live pointer.
pub fn release_freed_heap_report() -> (std::time::Duration, u64) {
    let at = std::time::Instant::now();
    #[cfg_attr(not(target_os = "ios"), allow(unused_mut))]
    let mut released = 0u64;
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        unsafe { malloc_trim(0) };
    }
    #[cfg(target_os = "ios")]
    {
        released = unsafe { malloc_zone_pressure_relief(core::ptr::null_mut(), 0) } as u64;
    }
    (at.elapsed(), released)
}

pub fn release_freed_heap() -> std::time::Duration {
    release_freed_heap_report().0
}

#[cfg(windows)]
static BACKING: mimalloc::MiMalloc = mimalloc::MiMalloc;
#[cfg(not(windows))]
static BACKING: std::alloc::System = std::alloc::System;

pub struct ProcessCountingAllocator;

unsafe impl GlobalAlloc for ProcessCountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        thread_net_add(layout.size() as i64);
        record_size(layout.size() as u64);
        record_alloc(layout.size() as u64);
        unsafe { BACKING.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        thread_net_add(layout.size() as i64);
        record_size(layout.size() as u64);
        record_alloc(layout.size() as u64);
        unsafe { BACKING.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        thread_net_add(-(layout.size() as i64));
        record_dealloc(layout.size() as u64);
        unsafe { BACKING.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        thread_net_add(new_size as i64 - layout.size() as i64);
        record_size(new_size as u64);
        record_alloc(new_size as u64);
        record_dealloc(layout.size() as u64);
        unsafe { BACKING.realloc(ptr, layout, new_size) }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
pub struct ProcessAllocationStats {
    pub process_allocations: u64,
    pub process_allocation_bytes: u64,
    pub process_deallocations: u64,
    pub process_deallocation_bytes: u64,

    pub process_allocation_saturated: u64,
}

pub fn process_allocations() -> ProcessAllocationStats {
    let n = NEXT_SLOT.load(Ordering::Relaxed).clamp(1, MAX_THREADS);
    let mut stats = ProcessAllocationStats::default();
    for slot in &SLOTS[..n] {
        stats.process_allocations += slot.alloc_count.load(Ordering::Relaxed);
        stats.process_allocation_bytes += slot.alloc_bytes.load(Ordering::Relaxed);
        stats.process_deallocations += slot.dealloc_count.load(Ordering::Relaxed);
        stats.process_deallocation_bytes += slot.dealloc_bytes.load(Ordering::Relaxed);
    }
    stats.process_allocation_saturated = SATURATED.load(Ordering::Relaxed);
    stats
}

pub fn process_live_heap_bytes() -> Option<u64> {
    if !counting_enabled() {
        return None;
    }
    let stats = process_allocations();
    Some(
        stats
            .process_allocation_bytes
            .saturating_sub(stats.process_deallocation_bytes),
    )
}

pub fn process_allocation_slots_used() -> usize {
    NEXT_SLOT.load(Ordering::Relaxed).clamp(1, MAX_THREADS)
}

pub fn process_allocation_saturated() -> u64 {
    SATURATED.load(Ordering::Relaxed)
}
