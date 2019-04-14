use std::{
    alloc::{alloc, alloc_zeroed, dealloc, handle_alloc_error, realloc, Layout},
    fmt,
    mem::{align_of, size_of},
    ptr::{self, NonNull},
};

use super::Core;


/// A `Core` implementation that is conceptually a `BitVec` and a `Vec<T>`.
///
/// TODO: explain advantages and disadvantages.
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
        // ∀ i in `new_len..vec.cap()` ⇒ `self.has_element_at(i) == false`

        self.len = new_len;
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
            // Build layout for allocation
            let new_elem_layout = {
                let len = match new_cap.checked_mul(size_of::<T>()) {
                    None => capacity_overflow(),
                    Some(len) => len,
                };

                Layout::from_size_align_unchecked(len, align_of::<T>())
            };

            let ptr = if self.cap == 0 {
                alloc(new_elem_layout)
            } else {
                realloc(
                    self.elem_ptr.as_ptr() as *mut _,
                    self.old_elem_layout(),
                    new_elem_layout.size(),
                )
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
            // TODO: remove
            let sum_before = (0..num_usizes_for(self.cap))
                .map(|i| *self.bit_ptr.as_ptr().add(i))
                .fold(0, |acc, i| acc ^ i);

            let new_bit_layout = Layout::from_size_align_unchecked(
                size_of::<usize>() * num_usizes_for(new_cap),
                align_of::<usize>(),
            );

            let ptr = if self.cap == 0 {
                alloc_zeroed(new_bit_layout)
            } else {
                realloc(
                    self.bit_ptr.as_ptr() as *mut _,
                    self.old_bit_layout(),
                    new_bit_layout.size(),
                )
            };
            let ptr = ptr as *mut usize;

            if ptr.is_null() {
                 handle_alloc_error(new_bit_layout);
            }

            if self.cap != 0 {
                // Zero out new bit blocks
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

            // TODO: remove
            let sum_after = (0..num_usizes_for(new_cap))
                .map(|i| *(ptr as *mut usize).add(i))
                .fold(0, |acc, i| acc ^ i);

            if sum_before != sum_after {
                panic!(
                    "incorrect bit realloc! before: {}, after: {}",
                    sum_before,
                    sum_after,
                );
            }

            self.bit_ptr = NonNull::new_unchecked(ptr as *mut _);
        }

        self.cap = new_cap;
    }

    unsafe fn has_element_at(&self, idx: usize) -> bool {
        debug_assert!(idx < self.cap());

        // The divisions will be turned into shift and and instructions
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

    unsafe fn next_index_from(&self, idx: usize) -> Option<usize> {
        debug_assert!(idx <= self.len());

        if idx >= self.len() {
            return None;
        }

        // Check current block
        let block_index = idx / BITS_PER_USIZE;
        let block = *self.bit_ptr.as_ptr().add(block_index);

        // Check for most common case: the index itself exists
        let bit_index = idx % BITS_PER_USIZE;
        let mask = 1 << bit_index;
        if (block & mask) != 0 {
            return Some(idx);
        }

        // Check the remaining current block for indices. We need to mask it to
        // avoid finding indices < idx. Example for bit_index 5:
        //
        //   mask:           0001 0000
        //   mask - 1:       0000 1111
        //   !(mask - 1):    1111 0000
        //   example block:  1010 0110
        //   and-result:     1010 0000   (trailing zeros: 5)
        let zeros = (block & !(mask - 1)).trailing_zeros() as usize;
        if zeros != size_of::<usize>() * 8 {
            return Some(block_index * BITS_PER_USIZE + zeros);
        }

        // Check for all other blocks
        let mut block_index = block_index + 1;
        while block_index < num_usizes_for(self.cap) {
            let block = *self.bit_ptr.as_ptr().add(block_index);

            // This makes seeking fast: we can very quickly check if there are
            // any bits set in this block.
            if block != 0 {
                return Some(block_index * BITS_PER_USIZE + block.trailing_zeros() as usize);
            }

            block_index += 1;
        }

        None
    }

    unsafe fn prev_index_from(&self, idx: usize) -> Option<usize> {
        debug_assert!(idx < self.len());

        (0..=idx)
            .rev()
            .find(|&idx| self.has_element_at(idx))
    }

    unsafe fn next_hole_from(&self, idx: usize) -> Option<usize> {
        debug_assert!(idx <= self.len());

        (idx..self.len)
            .find(|&idx| !self.has_element_at(idx))
    }

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
                    while let Some(next) = self.next_index_from(idx) {
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

#[inline(always)]
fn num_usizes_for(cap: usize) -> usize {
    // We need ⌈new_cap / BITS_PER_USIZE⌉ many usizes to store all required
    // bits. We do rounding up by first adding the (BITS_PER_USIZE - 1).
    (cap + (BITS_PER_USIZE - 1)) / BITS_PER_USIZE
}
