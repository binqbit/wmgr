use std::alloc::{GlobalAlloc, Layout, System};

use zeroize::Zeroize;

pub struct ZeroingAllocator;

unsafe impl GlobalAlloc for ZeroingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        System.alloc_zeroed(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if !ptr.is_null() && layout.size() != 0 {
            core::slice::from_raw_parts_mut(ptr, layout.size()).zeroize();
        }
        System.dealloc(ptr, layout);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ptr.is_null() {
            let Ok(new_layout) = Layout::from_size_align(new_size, layout.align()) else {
                return core::ptr::null_mut();
            };
            return System.alloc(new_layout);
        }

        if new_size == 0 {
            self.dealloc(ptr, layout);
            return core::ptr::null_mut();
        }

        let Ok(new_layout) = Layout::from_size_align(new_size, layout.align()) else {
            return core::ptr::null_mut();
        };

        let new_ptr = System.alloc(new_layout);
        if new_ptr.is_null() {
            return core::ptr::null_mut();
        }

        let copy_len = layout.size().min(new_size);
        if copy_len != 0 {
            core::ptr::copy_nonoverlapping(ptr, new_ptr, copy_len);
        }

        self.dealloc(ptr, layout);

        new_ptr
    }
}
