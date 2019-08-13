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
    /// The actual data, deleted flags and capacity.
    ///
    /// We chose this slightly strange representation for several reasons. For
    /// one, it looks like both fields could be replaced by a simple
    /// `Vec<Option<T>>`, but `Vec<T>` works a bit different than we need. We
    /// make exact reallocations and want to access all elements with indices
    /// smaller than the capacity. In short: we cannot simply use a
    /// `Vec<Option<T>>`.
    ///
    /// We need the `ManuallyDrop` mainly to make the `realloc` method easier
    /// and faster. In `drop()` we simply drop all elements that are `Some()`.
    data: Box<[ManuallyDrop<Option<T>>]>,

    /// The `len`: corresponse to the `len` of the `Core` definition.
    len: usize,
}

impl<T> Core<T> for OptionCore<T> {
    fn new() -> Self {
        Self {
            data: Box::default(),
            len: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    unsafe fn set_len(&mut self, v: usize) {
        debug_assert!(new_len <= self.cap());
        // Other precondition is too expensive to test, even in debug:
        // ∀ i in `new_len..self.cap()` ⇒ `self.has_element_at(i) == false`

        self.len = v;
    }

    fn cap(&self) -> usize {
        self.data.len()
    }

    #[inline(never)]
    #[cold]
    unsafe fn realloc(&mut self, new_cap: usize) {
        debug_assert!(new_cap >= self.len());
        debug_assert!(new_cap <= isize::max_value() as usize);

        let mut new: Vec<ManuallyDrop<Option<T>>> = Vec::with_capacity(new_cap);

        // Copy all old elements over to the new vector. After we do this, we
        // can just drop the box which will deallocate the old memory block,
        // but not touch the old values anymore (thanks to `ManuallyDrop`).
        let copy_len = cmp::min(self.data.len(), new_cap);
        ptr::copy_nonoverlapping(self.data.as_ptr(), new.as_mut_ptr(), copy_len);
        new.set_len(copy_len);

        // Fill the rest of the vector with deleted elements.
        new.resize_with(new_cap, || ManuallyDrop::new(None));

        self.data = new.into_boxed_slice();
    }

    unsafe fn has_element_at(&self, idx: usize) -> bool {
        debug_assert!(idx < self.cap());

        self.data.get_unchecked(idx).is_some()
    }

    unsafe fn insert_at(&mut self, idx: usize, elem: T) {
        debug_assert!(idx < self.cap());
        debug_assert!(self.has_element_at(idx) == false);


        *self.data.get_unchecked_mut(idx) = ManuallyDrop::new(Some(elem));
    }

    unsafe fn remove_at(&mut self, idx: usize) -> T {
        debug_assert!(idx < self.cap());
        debug_assert!(self.has_element_at(idx));

        match self.data.get_unchecked_mut(idx).deref_mut().take() {
            None => unreachable_unchecked(),
            Some(elem) => elem,
        }
    }

    unsafe fn get_unchecked(&self, idx: usize) -> &T {
        debug_assert!(idx < self.cap());
        debug_assert!(self.has_element_at(idx));

        match &**self.data.get_unchecked(idx) {
            None => unreachable_unchecked(),
            Some(elem) => elem,
        }
    }

    unsafe fn get_unchecked_mut(&mut self, idx: usize) -> &mut T {
        debug_assert!(idx < self.cap());
        debug_assert!(self.has_element_at(idx));

        match &mut **self.data.get_unchecked_mut(idx) {
            None => unreachable_unchecked(),
            Some(elem) => elem,
        }
    }

    fn clear(&mut self) {
        // We can assume that all existing elements have an index lower than
        // `len` (this is one of the invariants of the `Core` interface).
        for idx in 0..self.len {
            unsafe {
                // Call `Option::take` to overwrite everything with `None` and
                // drop all existing values.
                self.data.get_unchecked_mut(idx).deref_mut().take();
            }
        }
        self.len = 0;
    }

    unsafe fn next_index_from(&self, idx: usize) -> Option<usize> {
        debug_assert!(idx <= self.len());

        (idx..self.len)
            .find(|&idx| self.data.get_unchecked(idx).is_some())
    }

    unsafe fn prev_index_from(&self, idx: usize) -> Option<usize> {
        debug_assert!(idx < self.len());

        (0..=idx)
            .rev()
            .find(|&idx| self.data.get_unchecked(idx).is_some())
    }

    unsafe fn next_hole_from(&self, idx: usize) -> Option<usize> {
        debug_assert!(idx <= self.len());

        (idx..self.len)
            .find(|&idx| self.data.get_unchecked(idx).is_none())
    }

    unsafe fn swap(&mut self, a: usize, b: usize) {
        // We can't just have two mutable references, so we use `ptr::swap`
        // instead of `mem::swap`.
        let pa: *mut _ = self.data.get_unchecked_mut(a);
        let pb: *mut _ = self.data.get_unchecked_mut(b);
        ptr::swap(pa, pb);
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
            .field("len", &self.len())
            .field("cap", &self.cap())
            .finish()
    }
}
