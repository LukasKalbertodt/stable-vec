use std::{
    cmp,
    fmt,
    hint::unreachable_unchecked,
    mem::ManuallyDrop,
    ops::DerefMut,
    ptr,
};

use super::Core;

/// A `Core` implementation that is essentially a `Vec<Option<T>>`.
///
/// TODO: explain advantages and disadvantages.
#[derive(Clone)]
pub struct OptionCore<T> {
    data: Box<[ManuallyDrop<Option<T>>]>,
    used_len: usize,
}

impl<T> Core<T> for OptionCore<T> {
    fn new() -> Self {
        Self {
            data: Box::default(),
            used_len: 0,
        }
    }

    fn from_vec(vec: Vec<T>) -> Self {
        let mut out = Self::new();
        unsafe {
            // `vec.len()` is >= than `used_len` (0) here
            out.realloc(vec.len());
        }

        out.used_len = vec.len();
        for (i, elem) in vec.into_iter().enumerate() {
            // Due to the `grow` above we know that `i` is always greater than
            // `out.capacity()`. And because we started with an empty
            // instance, all elements start out as deleted.
            unsafe {
                out.insert_at(i, elem);
            }
        }

        out
    }

    fn used_len(&self) -> usize {
        self.used_len
    }

    unsafe fn set_used_len(&mut self, v: usize) {
        self.used_len = v;
    }

    fn capacity(&self) -> usize {
        self.data.len()
    }

    #[inline(never)]
    #[cold]
    unsafe fn realloc(&mut self, new_cap: usize) {
        // We at least double our capacity. Otherwise repeated `push`es are
        // O(n²).
        //
        // This multiplication can't overflow, because we know the capacity is
        // below `isize::MAX` (`Vec` ensures this).
        let new_cap = cmp::max(new_cap, 2 * self.capacity());

        let mut new: Vec<ManuallyDrop<Option<T>>> = Vec::with_capacity(new_cap);

        // Copy all old elements over to the new vector. After we do this, we
        // can just drop the box which will deallocate the old memory block,
        // but not touch the old values anymore (thanks to `ManuallyDrop`).
        unsafe {
            ptr::copy_nonoverlapping(self.data.as_ptr(), new.as_mut_ptr(), self.data.len());
            new.set_len(self.data.len());
        }

        // Fill the rest of the vector with deleted elements.
        new.resize_with(new_cap, || ManuallyDrop::new(None));

        self.data = new.into_boxed_slice();
    }

    unsafe fn has_element_at(&self, idx: usize) -> bool {
        self.data.get_unchecked(idx).is_some()
    }

    unsafe fn insert_at(&mut self, idx: usize, elem: T) {
        *self.data.get_unchecked_mut(idx) = ManuallyDrop::new(Some(elem));
    }

    unsafe fn remove_at(&mut self, idx: usize) -> T {
        match self.data.get_unchecked_mut(idx).deref_mut().take() {
            None => unreachable_unchecked(),
            Some(elem) => elem,
        }
    }

    unsafe fn get_unchecked(&self, idx: usize) -> &T {
        match &**self.data.get_unchecked(idx) {
            None => unreachable_unchecked(),
            Some(elem) => elem,
        }
    }

    unsafe fn get_unchecked_mut(&mut self, idx: usize) -> &mut T {
        match &mut **self.data.get_unchecked_mut(idx) {
            None => unreachable_unchecked(),
            Some(elem) => elem,
        }
    }

    fn clear(&mut self) {
        // We can assume that all existing elements have an index lower than
        // `used_len` (this is one of the invariants of the `Core` interface).
        for idx in 0..self.used_len {
            unsafe {
                // Call `Option::take` to overwrite everything with `None` and
                // drop all existing values.
                self.data.get_unchecked_mut(idx).deref_mut().take();
            }
        }
        self.used_len = 0;
    }

    fn next_index_from(&self, idx: usize) -> Option<usize> {
        (idx..self.used_len)
            .find(|&idx| self.data[idx].is_some())
    }

    fn prev_index_from(&self, idx: usize) -> Option<usize> {
        (0..=idx)
            .rev()
            .find(|&idx| self.data[idx].is_some())
    }
}

impl<T> Drop for OptionCore<T> {
    fn drop(&mut self) {
        // Drop all elements
        self.clear();
    }
}

// This impl is usually not used. `StableVec` has its own impl which doesn't
// use this one.
impl<T> fmt::Debug for OptionCore<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("OptionCore")
            .field("used_len", &self.used_len())
            .field("capacity", &self.capacity())
            .finish()
    }
}
