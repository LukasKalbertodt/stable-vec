use std::{
    alloc::{alloc, alloc_zeroed, dealloc, handle_alloc_error, realloc, Layout},
    fmt,
    mem::{align_of, size_of},
    ptr::{self, NonNull},
};

use super::Core;


/// A `Core` implementation that is conceptually a `BitVec` and a `Vec<T>`.
///
/// This is the default core as it has quite a few advantages. For one, it does
/// not waste memory due to padding. The information about whether a slot is
/// filled or not is stored in a `BitVec` and thus only takes one bit per slot.
///
/// Using a `BitVec` has another important advantage: iterating over the
/// indices of a stable vector (i.e. without touching the actual data) is very
/// cache-friendly. This is due to the dense packing of information. It's also
/// possible to very quickly scan the bit vector in order to find filled/empty
/// slots.
///
/// However, this core implementation has disadvantages, too. It manages two
/// allocations which means that reallocating (growing or shrinking) has to
/// perform two allocations with the underlying memory allocator. Potentially
/// more important is the decrease of cache-friendliness when accessing
/// elements at random. Because in the worst case, this means that each element
/// access results in two cache-misses instead of only one.
///
/// For most use cases, this is a good choice. That's why it's default.
pub struct BitVecCore<T> {
    /// This is the memory that stores the actual slots/elements. If a slot is
    /// empty, the memory at that index is undefined.
    elem_ptr: NonNull<T>,

    /// Stores whether or not slots are filled (1) or empty (0). Stores one bit
    /// per slot. Is stored as `usize` instead of `u8` to potentially improve
    /// performance when finding a hole or counting the elements.
    bit_ptr: NonNull<usize>,

    /// The capacity: the length of the `elem_ptr` buffer. Corresponse to the
    /// `cap` of the `Core` definition.
    cap: usize,

    /// The `len`: corresponse to the `len` of the `Core` definition.
    len: usize,
}

const BITS_PER_USIZE: usize = size_of::<usize>() * 8;

impl<T> BitVecCore<T> {
    /// Deallocates both pointers, sets them to the same value as `new()` does
    /// and sets `cap` to 0.
    ///
    /// # Formal
    ///
    /// **Preconditions**:
    /// - `self.len == 0`
    /// - All slots are empty
    unsafe fn dealloc(&mut self) {
        if self.cap != 0 {
            if size_of::<T>() != 0 {
                dealloc(self.elem_ptr.as_ptr() as *mut _, self.old_elem_layout());
            }

            dealloc(self.bit_ptr.as_ptr() as *mut _, self.old_bit_layout());
            self.cap = 0;
        }
    }

    /// Returns the layout that was used for the last allocation of `elem_ptr`.
    /// `self.cap` must not be 0 and `T` must not be a ZST, or else this
    /// method's behavior is undefined.
    unsafe fn old_elem_layout(&self) -> Layout {
        Layout::from_size_align_unchecked(
            // This can't overflow due to being previously allocated.
            self.cap * size_of::<T>(),
            align_of::<T>(),
        )
    }

    /// Returns the layout that was used for the last allocation of `bit_ptr`.
    /// `self.cap` must not be 0 or else this method's behavior is undefined.
    unsafe fn old_bit_layout(&self) -> Layout {
        Layout::from_size_align_unchecked(
            size_of::<usize>() * num_usizes_for(self.cap),
            align_of::<usize>(),
        )
    }
}

impl<T> Core<T> for BitVecCore<T> {
    fn new() -> Self {
        Self {
            elem_ptr: NonNull::dangling(),
            bit_ptr: NonNull::dangling(),
            cap: 0,
            len: 0,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.cap());
        // Other precondition is too expensive to test, even in debug:
        // ∀ i in `new_len..self.cap()` ⇒ `self.has_element_at(i) == false`

        self.len = new_len;

        // The formal requirements of this method hold:
        //
        // **Invariants**:
        // - *slot data* -> trivially holds, we do not touch that
        // - `len ≤ cap` -> that's a precondition
        //
        // **Postconditons**:
        // - `self.len() == new_len`: trivially holds
    }

    fn cap(&self) -> usize {
        self.cap
    }

    #[inline(never)]
    #[cold]
    unsafe fn realloc(&mut self, new_cap: usize) {
        debug_assert!(new_cap >= self.len());
        debug_assert!(new_cap <= isize::max_value() as usize);

        #[inline(never)]
        #[cold]
        fn capacity_overflow() -> ! {
            panic!("capacity overflow in `stable_vec::BitVecCore::realloc` (attempt \
                to allocate more than `usize::MAX` bytes");
        }

        // Handle special case
        if new_cap == 0 {
            // Due to preconditions, we know that `self.len == 0` and that in
            // turn tells us that there aren't any filled slots. So we can just
            // deallocate the memory.
            self.dealloc();
            return;
        }


        // ----- (Re)allocate element memory ---------------------------------

        // We only have to allocate if our size are not zero-sized. Else, we
        // just don't do anything.
        if size_of::<T>() != 0 {
            // Get the new number of bytes for the allocation and create the
            // memory layout.
            let size = new_cap.checked_mul(size_of::<T>())
                .unwrap_or_else(|| capacity_overflow());
            let new_elem_layout = Layout::from_size_align_unchecked(size, align_of::<T>());

            // (Re)allocate memory.
            let ptr = if self.cap == 0 {
                alloc(new_elem_layout)
            } else {
                realloc(self.elem_ptr.as_ptr() as *mut _, self.old_elem_layout(), size)
            };

            // If the element allocation failed, we quit the program with an
            // OOM error.
            if ptr.is_null() {
                 handle_alloc_error(new_elem_layout);
            }

            // We already overwrite the pointer here. It is not read/changed
            // anywhere else in this function.
            self.elem_ptr = NonNull::new_unchecked(ptr as *mut _);
        };


        // ----- (Re)allocate bitvec memory ----------------------------------
        {
            // Get the new number of required bytes for the allocation and
            // create the memory layout.
            let size = size_of::<usize>() * num_usizes_for(new_cap);
            let new_bit_layout = Layout::from_size_align_unchecked(size, align_of::<usize>());

            // (Re)allocate memory.
            let ptr = if self.cap == 0 {
                alloc_zeroed(new_bit_layout)
            } else {
                realloc(self.bit_ptr.as_ptr() as *mut _, self.old_bit_layout(), size)
            };
            let ptr = ptr as *mut usize;

            // If the element allocation failed, we quit the program with an
            // OOM error.
            if ptr.is_null() {
                 handle_alloc_error(new_bit_layout);
            }

            // If we reallocated, the new memory is not necessarily zeroed, so
            // we need to do it. TODO: if `alloc` offers a `realloc_zeroed`
            // in the future, we should use that.
            if self.cap != 0 {
                let initialized_usizes = num_usizes_for(self.cap);
                let new_usizes = num_usizes_for(new_cap);
                if new_usizes > initialized_usizes {
                    ptr::write_bytes(
                        ptr.add(initialized_usizes),
                        0,
                        new_usizes - initialized_usizes,
                    );
                }
            }

            self.bit_ptr = NonNull::new_unchecked(ptr as *mut _);
        }

        self.cap = new_cap;

        // All formal requirements are met now:
        //
        // **Invariants**:
        // - *slot data*: by using `realloc` if `self.cap != 0`, the slot data
        //   (including deleted-flag) was correctly copied.
        // - `self.len()`: indeed didn't change
        //
        // **Postconditons**:
        // - `self.cap() == new_cap`: trivially holds due to last line.
    }

    unsafe fn has_element_at(&self, idx: usize) -> bool {
        debug_assert!(idx < self.cap());

        // The divisions will be turned into shift and 'and'-instructions.
        let usize_pos = idx / BITS_PER_USIZE;
        let bit_pos = idx % BITS_PER_USIZE;

        let block = *self.bit_ptr.as_ptr().add(usize_pos);
        ((block >> bit_pos) & 0b1) != 0
    }

    unsafe fn insert_at(&mut self, idx: usize, elem: T) {
        debug_assert!(idx < self.cap());
        debug_assert!(self.has_element_at(idx) == false);

        // We first write the value and then update the bitvector to avoid
        // potential double drops if a random panic appears.
        ptr::write(self.elem_ptr.as_ptr().add(idx), elem);

        let usize_pos = idx / BITS_PER_USIZE;
        let bit_pos = idx % BITS_PER_USIZE;

        let mask = 1 << bit_pos;
        *self.bit_ptr.as_ptr().add(usize_pos) |= mask;
    }

    unsafe fn remove_at(&mut self, idx: usize) -> T {
        debug_assert!(idx < self.cap());
        debug_assert!(self.has_element_at(idx));

        // We first mark the value as deleted and then read the value.
        // Otherwise, a random panic could lead to a double drop.
        let usize_pos = idx / BITS_PER_USIZE;
        let bit_pos = idx % BITS_PER_USIZE;

        let mask = !(1 << bit_pos);
        *self.bit_ptr.as_ptr().add(usize_pos) &= mask;

        ptr::read(self.elem_ptr.as_ptr().add(idx))
    }

    unsafe fn get_unchecked(&self, idx: usize) -> &T {
        debug_assert!(idx < self.cap());
        debug_assert!(self.has_element_at(idx));

        // The preconditions of this function guarantees us that all
        // preconditions for `add` are met and that we can safely dereference
        // the pointer.
        &*self.elem_ptr.as_ptr().add(idx)
    }

    unsafe fn get_unchecked_mut(&mut self, idx: usize) -> &mut T {
        debug_assert!(idx < self.cap());
        debug_assert!(self.has_element_at(idx));

        // The preconditions of this function guarantees us that all
        // preconditions for `add` are met and that we can safely dereference
        // the pointer.
        &mut *self.elem_ptr.as_ptr().add(idx)
    }

    fn clear(&mut self) {
        unsafe {
            // We can assume that all existing elements have an index lower than
            // `len` (this is one of the invariants of the `Core` interface).
            for idx in 0..self.len {
                if self.has_element_at(idx) {
                    ptr::drop_in_place(self.get_unchecked_mut(idx));
                }
            }
            for bit_idx in 0..num_usizes_for(self.len) {
                *self.bit_ptr.as_ptr().add(bit_idx) = 0;
            }
            self.len = 0;
        }
    }

    // TODO: maybe override `{next|prev}_{hole|index}_from` for performance? In
    // principle we could scan the bitvector very quickly with specialized
    // instructions. Needs benchmarking.

    unsafe fn swap(&mut self, a: usize, b: usize) {
        // Swapping the bits is a bit annoying. To avoid branches we first xor
        // both previous bits.
        let a_existed = self.has_element_at(a);
        let b_existed = self.has_element_at(b);

        // `swap_bit` is 0 if both slots were empty of filled, and 1 if only
        // only one slot was empty. That also means the mask is 0 if both were
        // empty/filled before, otherwise the mask has one bit set. We xor with
        // this mask, meaning that we will flip the corresponding bit.
        let swap_bit = (a_existed ^ b_existed) as usize;

        // For a
        let usize_pos = a / BITS_PER_USIZE;
        let bit_pos = a % BITS_PER_USIZE;
        let mask = swap_bit << bit_pos;
        *self.bit_ptr.as_ptr().add(usize_pos) ^= mask;

        // For b
        let usize_pos = b / BITS_PER_USIZE;
        let bit_pos = b % BITS_PER_USIZE;
        let mask = swap_bit << bit_pos;
        *self.bit_ptr.as_ptr().add(usize_pos) ^= mask;

        // Finally swap the actual elements
        ptr::swap(
            self.elem_ptr.as_ptr().add(a),
            self.elem_ptr.as_ptr().add(b),
        );
    }
}

impl<T> Drop for BitVecCore<T> {
    fn drop(&mut self) {
        // Drop all elements
        self.clear();

        unsafe {
            // Deallocate the memory. `clear()` sets the length to 0 and drops
            // all existing elements, so it's fine to call `dealloc`.
            self.dealloc();
        }
    }
}

impl<T: Clone> Clone for BitVecCore<T> {
    fn clone(&self) -> Self {
        let mut out = Self::new();

        if self.cap != 0 {
            // All of this is scary
            unsafe {
                out.realloc(self.cap);

                // Copy element data over
                if size_of::<T>() != 0 {
                    let mut idx = 0;
                    while let Some(next) = self.first_filled_slot_from(idx) {
                        let clone = self.get_unchecked(next).clone();
                        ptr::write(out.elem_ptr.as_ptr().add(next), clone);

                        idx = next + 1;
                    }
                }

                // Copy bitvec data over
                ptr::copy_nonoverlapping(
                    self.bit_ptr.as_ptr(),
                    out.bit_ptr.as_ptr(),
                    num_usizes_for(self.cap)
                );

                out.set_len(self.len);
            }
        }

        out
    }
}

// This impl is usually not used. `StableVec` has its own impl which doesn't
// use this one.
impl<T> fmt::Debug for BitVecCore<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BitVecCore")
            .field("len", &self.len())
            .field("cap", &self.cap())
            .finish()
    }
}

// Implement `Send` and `Sync`. These are not automatically implemented as we
// use raw pointers. But they are safe to implement (given that `T` implements
// them). We do not have interior mutability, thus we can implement `Sync`. We
// also do not share any data with other instance of this type, meaning that
// `Send` can be implemented.
unsafe impl<T: Send> Send for BitVecCore<T> {}
unsafe impl<T: Sync> Sync for BitVecCore<T> {}

#[inline(always)]
fn num_usizes_for(cap: usize) -> usize {
    // We need ⌈new_cap / BITS_PER_USIZE⌉ many usizes to store all required
    // bits. We do rounding up by first adding the (BITS_PER_USIZE - 1).
    (cap + (BITS_PER_USIZE - 1)) / BITS_PER_USIZE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn num_usizes() {
        assert_eq!(num_usizes_for(0), 0);
        assert_eq!(num_usizes_for(1), 1);
        assert_eq!(num_usizes_for(2), 1);
        assert_eq!(num_usizes_for(3), 1);

        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(num_usizes_for(63), 1);
            assert_eq!(num_usizes_for(64), 1);
            assert_eq!(num_usizes_for(65), 2);
            assert_eq!(num_usizes_for(66), 2);
            assert_eq!(num_usizes_for(66), 2);

            assert_eq!(num_usizes_for(255), 4);
            assert_eq!(num_usizes_for(256), 4);
            assert_eq!(num_usizes_for(257), 5);
            assert_eq!(num_usizes_for(258), 5);
            assert_eq!(num_usizes_for(259), 5);
        }

        #[cfg(target_pointer_width = "32")]
        {
            assert_eq!(num_usizes_for(31), 1);
            assert_eq!(num_usizes_for(32), 1);
            assert_eq!(num_usizes_for(33), 2);
            assert_eq!(num_usizes_for(34), 2);
            assert_eq!(num_usizes_for(35), 2);

            assert_eq!(num_usizes_for(127), 4);
            assert_eq!(num_usizes_for(128), 4);
            assert_eq!(num_usizes_for(129), 5);
            assert_eq!(num_usizes_for(130), 5);
            assert_eq!(num_usizes_for(131), 5);
        }
    }
}
