// Copyright (C) 2026 Nguyen Hoang Quoc Anh
// Alias: Mon, Monbuticloud
// SPDX-License-Identifier: AGPL-3.0-only

mod app;
mod canvas;
mod document;
mod file_io;
mod files;
mod pixel;
mod tool_config;
mod ui;
mod undo;
mod undo_history;

#[cfg(test)]
mod tests;
use mimalloc::MiMalloc;
use directories::ProjectDirs;
use std::alloc::{ GlobalAlloc, Layout };
use std::sync::atomic::{ AtomicUsize, Ordering };

use crate::app::{ APP_QUALIFIER, APP_ORG, APP_NAME };

struct TrackingAllocator;

// real allocator underneath
static INNER_ALLOCATOR: MiMalloc = MiMalloc;

// live allocated bytes
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static TOTAL_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for TrackingAllocator {
    /// Allocate memory and track the live byte count.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr;
        unsafe {
            ptr = INNER_ALLOCATOR.alloc(layout);
        }

        if !ptr.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
            TOTAL_ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
            PEAK_ALLOCATED.fetch_max(ALLOCATED.load(Ordering::Relaxed), Ordering::Relaxed);
        }

        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe {
            INNER_ALLOCATOR.dealloc(ptr, layout);
        }

        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, old_layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr;
        unsafe {
            new_ptr = INNER_ALLOCATOR.realloc(ptr, old_layout, new_size);
        }

        if !new_ptr.is_null() {
            let old = old_layout.size();

            if new_size > old {
                ALLOCATED.fetch_add(new_size - old, Ordering::Relaxed);
                TOTAL_ALLOCATED.fetch_add(new_size - old, Ordering::Relaxed);
                PEAK_ALLOCATED.fetch_max(ALLOCATED.load(Ordering::Relaxed), Ordering::Relaxed);
            } else {
                ALLOCATED.fetch_sub(old - new_size, Ordering::Relaxed);
            }
        }

        new_ptr
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

pub fn allocated_bytes() -> usize {
    ALLOCATED.load(Ordering::Relaxed)
}

// Unstable MiMalloc features can cause build issues, so we'll stick to the default allocator for now.
// Never mind its just windows that has issues, linux and mac are fine. I'll just add a note about it in the readme (maybe) and leave it as is for now.

fn main() -> eframe::Result {
    let project_dirs = ProjectDirs::from(APP_QUALIFIER, APP_ORG, APP_NAME).expect(
        "Couldn't resolve app dir"
    );
    let data_dir = project_dirs.data_local_dir().to_path_buf();
    std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");

    let res = eframe::run_native(
        APP_NAME,
        eframe::NativeOptions::default(),
        Box::new(|_| Ok(Box::new(app::MyApp::default())))
    );
    println!("Total memory usage: {} bytes", TOTAL_ALLOCATED.load(Ordering::Relaxed));
    println!("Ending memory usage: {} bytes", ALLOCATED.load(Ordering::Relaxed));
    println!("Peak memory usage: {} bytes", PEAK_ALLOCATED.load(Ordering::Relaxed));
    res
}
