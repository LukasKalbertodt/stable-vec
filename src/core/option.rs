use std::{
    fmt,
    hint::unreachable_unchecked,
    ptr,
};

use super::Core;

/// A `Core` implementation that is essentially a `Vec<Option<T>>`.
///
/// TODO: explain advantages and disadvantages.
pub struct OptionCore<T> {
    /// The data and deleted information in one.
    ///
    /// The `len` and `capacity` properties of the vector directly correspond
    /// to `len` and `cap` properties of the `Core` trait. However, as a `Vec`
    /// assumes that everything beyond `len` is uninitialized, we have to make
    /// sure to only interact with it in a particular way.
    ///
    /// This vector is in a correct state at all times. This means that the
    /// vector can simply be dropped and it wouldn't access uninitialized
    /// values or leak memory.
    ///
    /// This implementation has one potentially problematic assumption. When we
    /// allocate new memory, we initialize all slots to `None`. That way we can
    /// access all slots with indices < cap. However, the `Vec` docs state:
    ///
    /// > Its uninitialized memory is scratch space that it may use however it
    /// > wants. It will generally just do whatever is most efficient or
    /// > otherwise easy to implement. [...] There is one case which we will
    /// > not break, however: using `unsafe` code to write to the excess
    /// > capacity, and then increasing the length to match, is always valid.
    ///
    /// This probably says that we cannot rely on the content of the excess
    /// capacity memory. However, we are careful how we touch the vector and we
    /// do not use any methods that would benefit in any way from touching that
    /// memory. Therefore we assume that all slots with indices > len stay
    /// initialized to `None`. A couple of methods rely on that assumption.
    data: Vec<Option<T>>,
}

impl<T> Core<T> for OptionCore<T> {
    fn new() -> Self {
        Self {
            data: Vec::new(),
        }
    }

    fn len(&self) -> usize {
        self.data.len()
    }

    fn cap(&self) -> usize {
        self.data.capacity()
    }

    unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.cap());
        // Other precondition is too expensive to test, even in debug:
        // ∀ i in `new_len..self.cap()` ⇒ `self.has_element_at(i) == false`

        // We can just call `set_len` on the vector as both of that method's
        // preconditions are held:
        // - "new_len must be less than or equal to capacity()": this is also a
        //   direct precondition of this method.
        // - "The elements at old_len..new_len must be initialized": all slots
        //   of the vector are always initialized. On allocation, everything is
        //   initialized to `None`. All slots in `old_len..new_len` are always
        //   `None` as stated by the `Core` invariant "`len ≤ i < cap`: slots
        //   with index `i` are always empty".
        self.data.set_len(new_len)
    }

    #[inline(never)]
    #[cold]
    unsafe fn realloc(&mut self, new_cap: usize) {
        debug_assert!(new_cap >= self.len());
        debug_assert!(new_cap <= isize::max_value() as usize);

        // Do different things depending on whether we shrink or grow.
        let old_cap = self.cap();
        let initialized_end = if new_cap > old_cap {
            // ----- We will grow the vector -----

            // We use `reserve_exact` here instead of creating a new vector,
            // because the former can use `realloc` which is significantly faster
            // in many cases. See https://stackoverflow.com/a/39562813/2408867
            let additional = new_cap - self.data.len();
            self.data.reserve_exact(additional);

            // `Vec` preserves all elements up to its length. Beyond that, the
            // slots might have become uninitialized by `reserve_exact`. Thus
            // we need to initialize them again.
            self.data.len()
        } else if new_cap < old_cap {
            // We will shrink the vector. The only tool we have for this is
            // `shrink_to_fit`. In order to use this, we temporarily have to
            // set the length of the vector to the new capacity. This is fine:
            //
            // - If `new_cap < old_len`, we temporarily remove elements from
            //   the vector. But these are all `None`s as guaranteed by the
            //   preconditions.
            // - If `new_cap > old_len`, we temporarily add elements to the
            //   vector. But these have all been initialized to `None`.
            let old_len = self.data.len();
            self.data.set_len(new_cap);
            self.data.shrink_to_fit();
            self.data.set_len(old_len);

            // When calling `shrink_to_fit`, the `Vec` cannot do anything funky
            // with the elements up to its size (which at that time was
            // `new_cap`). However, all memory that might exist beyond that
            // (i.e. if `shrink_to_fit` does not manage to perfectly fit) might
            // be uninitialized now.
            new_cap
        } else {
            // If the requested capacity is exactly the current one, we do
            // nothing. We return the current capacity from this expression to
            // say that all elements are indeed initialized.
            self.data.capacity()
        };

        // We now need to potentially initialize some elements to `None`. The
        // index `initialized_end` tells us the end of the range where all
        // elements are guaranteed to be initialized. Thus we need to
        // initialize `initialized_end..self.data.capacity()`.
        let actual_capacity = self.data.capacity();
        let mut ptr = self.data.as_mut_ptr().add(initialized_end);
        let end = self.data.as_mut_ptr().add(actual_capacity);
        while ptr != end {
            ptr::write(ptr, None);
            ptr = ptr.add(1);
        }
    }

    unsafe fn has_element_at(&self, idx: usize) -> bool {
        debug_assert!(idx < self.cap());

        self.data.get_unchecked(idx).is_some()
    }

    unsafe fn insert_at(&mut self, idx: usize, elem: T) {
        debug_assert!(idx < self.cap());
        debug_assert!(self.has_element_at(idx) == false);

        // We use `ptr::write` instead of a simple assignment here for
        // performance reason. An assignment would try to drop the value on the
        // left hand side. Since we know from our preconditions that this value
        // is in fact `None` and we thus never need to drop it, `ptr::write` is
        // faster.
        ptr::write(self.data.get_unchecked_mut(idx), Some(elem));
    }

    unsafe fn remove_at(&mut self, idx: usize) -> T {
        debug_assert!(idx < self.cap());
        debug_assert!(self.has_element_at(idx));

        match self.data.get_unchecked_mut(idx).take() {
            // The precondition guarantees us that the slot is not empty, thus
            // we use this unsafe `unreachable_unchecked` to omit the branch.
            None => unreachable_unchecked(),
            Some(elem) => elem,
        }
    }

    unsafe fn get_unchecked(&self, idx: usize) -> &T {
        debug_assert!(idx < self.cap());
        debug_assert!(self.has_element_at(idx));

        match self.data.get_unchecked(idx) {
            // The precondition guarantees us that the slot is not empty, thus
            // we use this unsafe `unreachable_unchecked` to omit the branch.
            None => unreachable_unchecked(),
            Some(elem) => elem,
        }
    }

    unsafe fn get_unchecked_mut(&mut self, idx: usize) -> &mut T {
        debug_assert!(idx < self.cap());
        debug_assert!(self.has_element_at(idx));

        match self.data.get_unchecked_mut(idx) {
            // The precondition guarantees us that the slot is not empty, thus
            // we use this unsafe `unreachable_unchecked` to omit the branch.
            None => unreachable_unchecked(),
            Some(elem) => elem,
        }
    }

    fn clear(&mut self) {
        // We can assume that all existing elements have an index lower than
        // `len` (this is one of the invariants of the `Core` interface).
        // Calling `clear` on the `Vec` will drop all remaining elements and
        // sets the length to 0.
        self.data.clear();
    }

    unsafe fn swap(&mut self, a: usize, b: usize) {
        // We can't just have two mutable references, so we use `ptr::swap`
        // instead of `mem::swap`. We do not use the slice's `swap` method as
        // that performs bound checks.
        let pa: *mut _ = self.data.get_unchecked_mut(a);
        let pb: *mut _ = self.data.get_unchecked_mut(b);
        ptr::swap(pa, pb);
    }
}

impl<T: Clone> Clone for OptionCore<T> {
    fn clone(&self) -> Self {
        // Cloning the vector is safe: the `Vec` implementation won't access
        // uninitialized memory. However, simply cloning it would be wrong for
        // two reasons:
        //
        // - `Vec` might not retain the same capacity when cloning it. But this
        //   is important for us.
        // - The memory after its length is probably uninitialized.
        //
        // To fix both issues, we get a slice to the complete memory of the
        // original `Vec` and create a `Vec` from it. Then we reset the length
        // to the old value. Both is safe as all the elements that are included
        // and excluded by the "fake length" are `None`.
        let data = unsafe {
            let mut data_clone = self.data.get_unchecked(0..self.data.capacity()).to_vec();
            data_clone.set_len(self.data.len());
            data_clone
        };

        Self { data }
    }
}

impl<T> Drop for OptionCore<T> {
    fn drop(&mut self) {
        // We don't need to anything! The `Vec` will be dropped which is
        // correct: that will drop all remaining elements but won't touch
        // non-existing elements. This manual `Drop` impl still exists to
        // explain this fact and to make sure the automatic `Drop` impl won't
        // lead to unsafety in the future.
    }
}

// This impl is usually not used. `StableVec` has its own impl which doesn't
// use this one.
impl<T: fmt::Debug> fmt::Debug for OptionCore<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("OptionCore")
            .field(&self.data)
            .finish()
    }
}
