//! A `Vec<T>`-like collection which guarantees stable indices and features
//! O(1) deletion of elements.
//!
//! You can find nearly all the relevant documentation on the type
//! [`StableVecFacade`]. This is the main type which is configurable over the
//! core implementation. To use a pre-configured stable vector, use
//! [`StableVec`].
//!
//!
//! # Why?
//!
//! The standard `Vec<T>` always stores all elements contiguously. While this
//! has many advantages (most notable: cache friendliness), it has the
//! disadvantage that you can't simply remove an element from the middle; at
//! least not without shifting all elements after it to the left. And this has
//! two major drawbacks:
//!
//! 1. It has a linear O(n) time complexity
//! 2. It invalidates all indices of the shifted elements
//!
//! Invalidating an index means that a given index `i` who referred to an
//! element `a` before, now refers to another element `b`. On the contrary, a
//! *stable* index means, that the index always refers to the same element.
//!
//! Stable indices are needed in quite a few situations. One example are graph
//! data structures (or complex data structures in general). Instead of
//! allocating heap memory for every node and edge, all nodes and all edges are
//! stored in a vector (each). But how does the programmer unambiguously refer
//! to one specific node? A pointer is not possible due to the reallocation
//! strategy of most dynamically growing arrays (the pointer itself is not
//! *stable*). Thus, often the index is used.
//!
//! But in order to use the index, it has to be stable. This is one example,
//! where this data structure comes into play.
//!
//!
//! # How?
//!
//! Actually, the implementation of this stable vector is rather simple. We can
//! trade O(1) deletions and stable indices for a higher memory consumption.
//!
//! When `StableVec::remove()` is called, the element is just marked as
//! "deleted" (and the actual element is dropped), but other than that, nothing
//! happens. This has the very obvious disadvantage that deleted objects (so
//! called empty slots) just waste space. This is also the most important thing
//! to understand:
//!
//! The memory requirement of this data structure is `O(|inserted elements|)`;
//! instead of `O(|inserted elements| - |removed elements|)`. The latter is the
//! memory requirement of normal `Vec<T>`. Thus, if deletions are far more
//! numerous than insertions in your situation, then this data structure is
//! probably not fitting your needs.
//!
//!
//! # Why not?
//!
//! As mentioned above, this data structure is rather simple and has many
//! disadvantages on its own. Here are some reason not to use it:
//!
//! - You don't need stable indices or O(1) removal
//! - Your deletions significantly outnumber your insertions
//! - You want to choose your keys/indices
//! - Lookup times do not matter so much to you
//!
//! Especially in the last two cases, you could consider using a `HashMap` with
//! integer keys, best paired with a fast hash function for small keys.
//!
//! If you not only want stable indices, but stable pointers, you might want
//! to use something similar to a linked list. Although: think carefully about
//! your problem before using a linked list.
//!
#![deny(missing_debug_implementations)]
#![deny(intra_doc_link_resolution_failure)]


use std::{
    cmp,
    fmt,
    io,
    iter::FromIterator,
    mem,
    ops::{Index, IndexMut},
};
use crate::{
    core::{Core, DefaultCore, OwningCore, OptionCore, BitVecCore},
    iter::{Indices, Iter, IterMut, IntoIter},
};

#[cfg(test)]
mod tests;
pub mod core;
pub mod iter;



/// A stable vector with the default core implementation.
pub type StableVec<T> = StableVecFacade<T, DefaultCore<T>>;

/// A stable vector which stores the "deleted information" inline. This is very
/// close to `Vec<Option<T>>`.
///
/// This is particularly useful if `T` benefits from "null optimization", i.e.
/// if `size_of::<T>() == size_of::<Option<T>>()`.
pub type InlineStableVec<T> = StableVecFacade<T, OptionCore<T>>;

/// A stable vector which stores the "deleted information" externally in a bit
/// vector.
pub type ExternStableVec<T> = StableVecFacade<T, BitVecCore<T>>;


/// A `Vec<T>`-like collection which guarantees stable indices and features
/// O(1) deletion of elements.
///
///
/// # Terminology and overview of a stable vector
///
/// A stable vector has slots. Each slot can either be filled or empty. There
/// are three numbers describing a stable vector (each of those functions runs
/// in O(1)):
///
/// - [`capacity()`][StableVecFacade::capacity]: the total number of slots
///   (filled and empty).
/// - [`num_elements()`][StableVecFacade::num_elements]: the number of filled
///   slots.
/// - [`next_push_index()`][StableVecFacade::next_push_index]: the index of the
///   first slot (i.e. with the smallest index) that was never filled. This is
///   the index that is returned by [`push`][StableVecFacade::push]. This
///   implies that all filled slots have indices smaller than
///   `next_push_index()`.
///
/// Here is an example visualization (with `num_elements = 4`).
///
/// ```text
///      0   1   2   3   4   5   6   7   8   9   10
///    ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
///    │ a │ - │ b │ c │ - │ - │ d │ - │ - │ - │
///    └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
///                                      ↑       ↑
///                        next_push_index       capacity
/// ```
///
/// Unlike `Vec<T>`, `StableVecFacade` allows access to all slots with indices
/// between 0 and `capacity()`. In particular, it is allowed to call
/// [`insert`][StableVecFacade::insert] with all indices smaller than
/// `capacity()`.
///
///
/// # The Core implementation `C`
///
/// You might have noticed the type parameter `C`. There are actually multiple
/// ways how to implement the abstact data structure described above. One might
/// basically use a `Vec<Option<T>>`. But there are other ways, too.
///
/// Most of the time, you can simply use the alias [`StableVec`] which uses the
/// [`DefaultCore`]. This is fine for almost all cases. That's why all
/// documentation examples use that type instead of the generic
/// `StableVecFacade`.
///
/// <br>
/// <br>
/// <br>
/// <br>
///
/// # Method overview
///
/// (*there are more methods than mentioned in this overview*)
///
/// **Creating a stable vector**
///
/// - [`new`][StableVecFacade::new]
/// - [`with_capacity`][StableVecFacade::with_capacity]
/// - [`FromIterator::from_iter`](#impl-FromIterator<T>)
///
/// **Adding and removing elements**
///
/// - [`push`][StableVecFacade::push]
/// - [`insert`][StableVecFacade::insert]
/// - [`remove`][StableVecFacade::remove]
/// - [`remove_last`][StableVecFacade::remove_last]
/// - [`remove_first`][StableVecFacade::remove_first]
///
/// **Accessing elements**
///
/// - [`get`][StableVecFacade::get] (returns `Option<&T>`)
/// - [the `[]` index operator](#impl-Index<usize>) (returns `&T`)
/// - [`get_mut`][StableVecFacade::get_mut] (returns `Option<&mut T>`)
/// - [the mutable `[]` index operator](#impl-IndexMut<usize>) (returns `&mut T`)
/// - [`remove`][StableVecFacade::remove] (returns `Option<T>`)
///
/// **Stable vector specific**
///
/// - [`has_element_at`][StableVecFacade::has_element_at]
/// - [`next_push_index`][StableVecFacade::next_push_index]
/// - [`is_compact`][StableVecFacade::is_compact]
/// - [`make_compact`][StableVecFacade::make_compact]
/// - [`reordering_make_compact`][StableVecFacade::reordering_make_compact]
///
/// **Number of elements**
///
/// - [`is_empty`][StableVecFacade::is_empty]
/// - [`num_elements`][StableVecFacade::num_elements]
///
/// **Capacity management**
///
/// - [`capacity`][StableVecFacade::capacity]
/// - [`reserve`][StableVecFacade::reserve]
/// - [`reserve_for`][StableVecFacade::reserve_for]
/// - [`reserve_exact`][StableVecFacade::reserve_exact]
/// - [`shrink_to_fit`][StableVecFacade::shrink_to_fit]
///
// #[derive(PartialEq, Eq)]
#[derive(Clone)]
pub struct StableVecFacade<T, C: Core<T>> {
    core: OwningCore<T, C>,
    num_elements: usize,
}

impl<T, C: Core<T>> StableVecFacade<T, C> {
    /// Constructs a new, empty stable vector.
    ///
    /// The stable-vector will not allocate until elements are pushed onto it.
    pub fn new() -> Self {
        Self {
            core: OwningCore::new(C::new()),
            num_elements: 0,
        }
    }

    /// Constructs a new, empty stable vector with the specified capacity.
    ///
    /// The stable-vector will be able to hold exactly `capacity` elements
    /// without reallocating. If `capacity` is 0, the stable-vector will not
    /// allocate any memory. See [`reserve`][StableVecFacade::reserve] for more
    /// information.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut out = Self::new();
        out.reserve_exact(capacity);
        out
    }

    /// Reserves memory for at least `additional` more elements to be inserted
    /// at indices `>= self.next_push_index()`.
    ///
    /// This method might allocate more than `additional` to avoid frequent
    /// reallocations. Does nothing if the current capacity is already
    /// sufficient. After calling this method, `self.capacity()` is ≥
    /// `self.next_push_index() + additional`.
    ///
    /// Unlike `Vec::reserve`, the additional reserved memory is not completely
    /// unaccessible. Instead, additional empty slots are added to this stable
    /// vector. These can be used just like any other empty slot; in
    /// particular, you can insert into it.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::new();
    /// let star_idx = sv.push('★');
    ///
    /// // After we inserted one element, the next element would sit at index
    /// // 1, as expected.
    /// assert_eq!(sv.next_push_index(), 1);
    ///
    /// sv.reserve(2); // insert two empty slots
    ///
    /// // `reserve` doesn't change any of this
    /// assert_eq!(sv.num_elements(), 1);
    /// assert_eq!(sv.next_push_index(), 1);
    ///
    /// // We can now insert an element at index 2.
    /// sv.insert(2, 'x');
    /// assert_eq!(sv[2], 'x');
    ///
    /// // These values get adjusted accordingly.
    /// assert_eq!(sv.num_elements(), 2);
    /// assert_eq!(sv.next_push_index(), 3);
    /// ```
    pub fn reserve(&mut self, additional: usize) {
        #[inline(never)]
        #[cold]
        fn capacity_overflow() -> ! {
            panic!("capacity overflow in `stable_vec::StableVecFacade::reserve` (attempt \
                to allocate more than `isize::MAX` elements");
        }

        //:    new_cap = len + additional  ∧  additional >= 0
        //: => new_cap >= len
        let new_cap = match self.core.len().checked_add(additional) {
            None => capacity_overflow(),
            Some(new_cap) => new_cap,
        };

        if self.core.cap() < new_cap {
            // We at least double our capacity. Otherwise repeated `push`es are
            // O(n²).
            //
            // This multiplication can't overflow, because we know the capacity
            // is `<= isize::MAX`.
            //
            //:    new_cap = max(new_cap_before, 2 * cap)
            //:        ∧ cap >= len
            //:        ∧ new_cap_before >= len
            //: => new_cap >= len
            let new_cap = cmp::max(new_cap, 2 * self.core.cap());

            if new_cap > isize::max_value() as usize {
                capacity_overflow();
            }

            //: new_cap >= len  ∧  new_cap <= isize::MAX
            //
            // These both properties are exactly the preconditions of
            // `realloc`, so we can safely call that method.
            unsafe {
                self.core.realloc(new_cap);
            }
        }
    }

    /// Reserve enough memory so that there is a slot at `index`. Does nothing
    /// if `index < self.capacity()`.
    ///
    /// This method might allocate more memory than requested to avoid frequent
    /// allocations. After calling this method, `self.capacity() >= index + 1`.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::new();
    /// let star_idx = sv.push('★');
    ///
    /// // Allocate enough memory so that we have a slot at index 5.
    /// sv.reserve_for(5);
    /// assert!(sv.capacity() >= 6);
    ///
    /// // We can now insert an element at index 5.
    /// sv.insert(5, 'x');
    /// assert_eq!(sv[5], 'x');
    ///
    /// // This won't do anything as the slot with index 3 already exists.
    /// let capacity_before = sv.capacity();
    /// sv.reserve_for(3);
    /// assert_eq!(sv.capacity(), capacity_before);
    /// ```
    pub fn reserve_for(&mut self, index: usize) {
        if index >= self.capacity() {
            // Won't underflow as `index >= capacity >= next_push_index`.
            self.reserve(1 + index - self.next_push_index());
        }
    }

    /// Like [`reserve`][StableVecFacade::reserve], but tries to allocate
    /// memory for exactly `additional` more elements.
    ///
    /// The underlying allocator might allocate more memory than requested,
    /// meaning that you cannot rely on the capacity of this stable vector
    /// having an exact value after calling this method.
    pub fn reserve_exact(&mut self, additional: usize) {
        #[inline(never)]
        #[cold]
        fn capacity_overflow() -> ! {
            panic!("capacity overflow in `stable_vec::StableVecFacade::reserve_exact` (attempt \
                to allocate more than `isize::MAX` elements");
        }

        //:    new_cap = len + additional  ∧  additional >= 0
        //: => new_cap >= len
        let new_cap = match self.core.len().checked_add(additional) {
            None => capacity_overflow(),
            Some(new_cap) => new_cap,
        };

        if self.core.cap() < new_cap {
            if new_cap > isize::max_value() as usize {
                capacity_overflow();
            }

            //: new_cap >= len  ∧  new_cap <= isize::MAX
            //
            // These both properties are exactly the preconditions of
            // `realloc`, so we can safely call that method.
            unsafe {
                self.core.realloc(new_cap);
            }
        }
    }

    /// Inserts the new element `elem` at index `self.next_push_index` and
    /// returns said index.
    ///
    /// The inserted element will always be accessible via the returned index.
    ///
    /// This method has an amortized runtime complexity of O(1), just like
    /// `Vec::push`.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::new();
    /// let star_idx = sv.push('★');
    /// let heart_idx = sv.push('♥');
    ///
    /// assert_eq!(sv.get(heart_idx), Some(&'♥'));
    ///
    /// // After removing the star we can still use the heart's index to access
    /// // the element!
    /// sv.remove(star_idx);
    /// assert_eq!(sv.get(heart_idx), Some(&'♥'));
    /// ```
    pub fn push(&mut self, elem: T) -> usize {
        let index = self.core.len();
        self.reserve(1);

        unsafe {
            // Due to `reserve`, the core holds at least one empty slot, so we
            // know that `index` is smaller than the capacity. We also know
            // that at `index` there is no element (the definition of `len`
            // guarantees this).
            self.core.set_len(index + 1);
            self.core.insert_at(index, elem);
        }

        self.num_elements += 1;
        index
    }

    /// Inserts the given value at the given index.
    ///
    /// If the slot at `index` is empty, the `elem` is inserted at that
    /// position and `None` is returned. If there is an existing element `x` at
    /// that position, that element is replaced by `elem` and `Some(x)` is
    /// returned. The `next_push_index` is adjusted accordingly if `index >=
    /// next_push_index()`.
    ///
    ///
    /// # Panics
    ///
    /// Panics if the index is `>= self.capacity()`.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::new();
    /// let star_idx = sv.push('★');
    /// let heart_idx = sv.push('♥');
    ///
    /// // Inserting into an empty slot (element was deleted).
    /// sv.remove(star_idx);
    /// assert_eq!(sv.num_elements(), 1);
    /// assert_eq!(sv.insert(star_idx, 'x'), None);
    /// assert_eq!(sv.num_elements(), 2);
    /// assert_eq!(sv[star_idx], 'x');
    ///
    /// // We can also reserve memory (create new empty slots) and insert into
    /// // such a new slot. Note that that `next_push_index` gets adjusted.
    /// sv.reserve_for(5);
    /// assert_eq!(sv.insert(5, 'y'), None);
    /// assert_eq!(sv.num_elements(), 3);
    /// assert_eq!(sv.next_push_index(), 6);
    /// assert_eq!(sv[5], 'y');
    ///
    /// // Inserting into a filled slot replaces the value and returns the old
    /// // value.
    /// assert_eq!(sv.insert(heart_idx, 'z'), Some('♥'));
    /// assert_eq!(sv[heart_idx], 'z');
    /// ```
    pub fn insert(&mut self, index: usize, mut elem: T) -> Option<T> {
        // If the index is out of bounds, we cannot insert the new element.
        if index >= self.core.cap() {
            panic!(
                "`index ({}) >= capacity ({})` in `StableVecFacade::insert`",
                index,
                self.core.cap(),
            );
        }

        if self.has_element_at(index) {
            unsafe {
                // We just checked there is an element at that position, so
                // this is fine.
                mem::swap(self.core.get_unchecked_mut(index), &mut elem);
            }
            Some(elem)
        } else {
            if index >= self.core.len() {
                // Due to the bounds check above, we know that `index + 1` is ≤
                // `capacity`.
                unsafe {
                    self.core.set_len(index + 1);
                }
            }

            unsafe {
                // `insert_at` requires that `index < cap` and
                // `!has_element_at(index)`. Both of these conditions are met
                // by the two explicit checks above.
                self.core.insert_at(index, elem);
            }

            self.num_elements += 1;

            None
        }
    }

    /// Removes and returns the first element from this collection, or `None`
    /// if it's empty.
    ///
    /// This method uses exactly the same deletion strategy as
    /// [`remove()`][StableVecFacade::remove].
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[1, 2, 3]);
    /// assert_eq!(sv.remove_first(), Some(1));
    /// assert_eq!(sv, vec![2, 3]);
    /// ```
    ///
    /// # Note
    ///
    /// This method needs to find the index of the first valid element. Finding
    /// it has a worst case time complexity of O(n). If you already know the
    /// index, use [`remove()`][StableVecFacade::remove] instead.
    pub fn remove_first(&mut self) -> Option<T> {
        self.find_first_index().and_then(|index| self.remove(index))
    }

    /// Removes and returns the last element from this collection, or `None` if
    /// it's empty.
    ///
    /// This method uses exactly the same deletion strategy as
    /// [`remove()`][StableVecFacade::remove].
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[1, 2, 3]);
    /// assert_eq!(sv.remove_last(), Some(3));
    /// assert_eq!(sv, vec![1, 2]);
    /// ```
    ///
    /// # Note
    ///
    /// This method needs to find the index of the last valid element. Finding
    /// it has a worst case time complexity of O(n). If you already know the
    /// index, use [`remove()`][StableVecFacade::remove] instead.
    pub fn remove_last(&mut self) -> Option<T> {
        self.find_last_index().and_then(|index| self.remove(index))
    }

    /// Finds the first element and returns a reference to it, or `None` if
    /// the stable vector is empty.
    ///
    /// This method has a worst case time complexity of O(n).
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[1, 2]);
    /// sv.remove(0);
    /// assert_eq!(sv.find_first(), Some(&2));
    /// ```
    pub fn find_first(&self) -> Option<&T> {
        self.find_first_index().map(|index| unsafe { self.core.get_unchecked(index) })
    }

    /// Finds the first element and returns a mutable reference to it, or
    /// `None` if the stable vector is empty.
    ///
    /// This method has a worst case time complexity of O(n).
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[1, 2]);
    /// {
    ///     let first = sv.find_first_mut().unwrap();
    ///     assert_eq!(*first, 1);
    ///
    ///     *first = 3;
    /// }
    /// assert_eq!(sv, vec![3, 2]);
    /// ```
    pub fn find_first_mut(&mut self) -> Option<&mut T> {
        self.find_first_index().map(move |index| unsafe { self.core.get_unchecked_mut(index) })
    }

    /// Finds the last element and returns a reference to it, or `None` if
    /// the stable vector is empty.
    ///
    /// This method has a worst case time complexity of O(n).
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[1, 2]);
    /// sv.remove(1);
    /// assert_eq!(sv.find_last(), Some(&1));
    /// ```
    pub fn find_last(&self) -> Option<&T> {
        self.find_last_index().map(|index| unsafe { self.core.get_unchecked(index) })
    }

    /// Finds the last element and returns a mutable reference to it, or `None`
    /// if the stable vector is empty.
    ///
    /// This method has a worst case time complexity of O(n).
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[1, 2]);
    /// {
    ///     let last = sv.find_last_mut().unwrap();
    ///     assert_eq!(*last, 2);
    ///
    ///     *last = 3;
    /// }
    /// assert_eq!(sv, vec![1, 3]);
    /// ```
    pub fn find_last_mut(&mut self) -> Option<&mut T> {
        self.find_last_index().map(move |index| unsafe { self.core.get_unchecked_mut(index) })
    }

    /// Finds the first element and returns its index, or `None` if the stable
    /// vector is empty.
    ///
    /// This method has a worst case time complexity of O(n).
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[1, 2]);
    /// sv.remove(0);
    /// assert_eq!(sv.find_first_index(), Some(1));
    /// ```
    pub fn find_first_index(&self) -> Option<usize> {
        unsafe {
            self.core.next_index_from(0)
        }
    }

    /// Finds the last element and returns its index, or `None` if the stable
    /// vector is empty.
    ///
    /// This method has a worst case time complexity of O(n).
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[1, 2]);
    /// sv.remove(1);
    /// assert_eq!(sv.find_last_index(), Some(0));
    /// ```
    pub fn find_last_index(&self) -> Option<usize> {
        let len = self.core.len();
        if len == 0 {
            None
        } else {
            unsafe {
                self.core.prev_index_from(len - 1)
            }
        }
    }

    /// Removes and returns the element at position `index`. If the slot at
    /// `index` is empty, nothing is changed and `None` is returned.
    ///
    /// This simply marks the slot at `index` as empty. The elements after the
    /// given index are **not** shifted to the left. Thus, the time complexity
    /// of this method is O(1).
    ///
    /// # Panic
    ///
    /// Panics if `index >= self.capacity()`.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::new();
    /// let star_idx = sv.push('★');
    /// let heart_idx = sv.push('♥');
    ///
    /// assert_eq!(sv.remove(star_idx), Some('★'));
    /// assert_eq!(sv.remove(star_idx), None); // the star was already removed
    ///
    /// // We can use the heart's index here. It has not been invalidated by
    /// // the removal of the star.
    /// assert_eq!(sv.remove(heart_idx), Some('♥'));
    /// assert_eq!(sv.remove(heart_idx), None); // the heart was already removed
    /// ```
    pub fn remove(&mut self, index: usize) -> Option<T> {
        // If the index is out of bounds, we cannot insert the new element.
        if index >= self.core.cap() {
            panic!(
                "`index ({}) >= capacity ({})` in `StableVecFacade::remove`",
                index,
                self.core.cap(),
            );
        }

        if self.has_element_at(index) {
            // We checked with `Self::has_element_at` that the conditions for
            // `remove_at` are met.
            let elem = unsafe {
                self.core.remove_at(index)
            };

            self.num_elements -= 1;
            Some(elem)
        } else {
            None
        }
    }

    /// Returns a reference to the element at the given index, or `None` if
    /// there exists no element at that index.
    ///
    /// If you are calling `unwrap()` on the result of this method anyway,
    /// rather use the index operator instead: `stable_vec[index]`.
    pub fn get(&self, index: usize) -> Option<&T> {
        if self.has_element_at(index) {
            // We might call this, because we checked both conditions via
            // `Self::has_element_at`.
            let elem = unsafe {
                self.core.get_unchecked(index)
            };
            Some(elem)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the element at the given index, or
    /// `None` if there exists no element at that index.
    ///
    /// If you are calling `unwrap()` on the result of this method anyway,
    /// rather use the index operator instead: `stable_vec[index]`.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if self.has_element_at(index) {
            // We might call this, because we checked both conditions via
            // `Self::has_element_at`.
            let elem = unsafe {
                self.core.get_unchecked_mut(index)
            };
            Some(elem)
        } else {
            None
        }
    }

    /// Returns a reference to the element at the given index without checking
    /// the index.
    ///
    /// # Security
    ///
    /// When calling this method `self.has_element_at(index)` has to be `true`,
    /// otherwise this method's behavior is undefined! This requirement implies
    /// the requirement `index < self.next_push_index()`.
    pub unsafe fn get_unchecked(&self, index: usize) -> &T {
        self.core.get_unchecked(index)
    }

    /// Returns a mutable reference to the element at the given index without
    /// checking the index.
    ///
    /// # Security
    ///
    /// When calling this method `self.has_element_at(index)` has to be `true`,
    /// otherwise this method's behavior is undefined! This requirement implies
    /// the requirement `index < self.next_push_index()`.
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> &mut T {
        self.core.get_unchecked_mut(index)
    }

    /// Returns `true` if there exists an element at the given index (i.e. the
    /// slot at `index` is *not* empty), `false` otherwise.
    ///
    /// An element is said to exist if the index is not out of bounds and the
    /// slot at the given index is not empty. In particular, this method can
    /// also be called with indices larger than the current capacity (although,
    /// `false` is always returned in those cases).
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::new();
    /// assert!(!sv.has_element_at(3));         // no: index out of bounds
    ///
    /// let heart_idx = sv.push('♥');
    /// assert!(sv.has_element_at(heart_idx));  // yes
    ///
    /// sv.remove(heart_idx);
    /// assert!(!sv.has_element_at(heart_idx)); // no: was removed
    /// ```
    pub fn has_element_at(&self, index: usize) -> bool {
        if index >= self.core.cap() {
            false
        } else {
            unsafe {
                // The index is smaller than the capacity, as checked aboved,
                // so we can call this without a problem.
                self.core.has_element_at(index)
            }
        }
    }

    /// Reallocates to have a capacity as small as possible while still holding
    /// `self.next_push_index()` slots.
    ///
    /// Note that this does not move existing elements around and thus does not
    /// invalidate indices. This method also doesn't change what
    /// `next_push_index` returns. Instead, only the capacity is changed. Due
    /// to the underlying allocator, it cannot be guaranteed that the capacity
    /// is exactly `self.next_push_index()` after calling this method.
    ///
    /// If you want to compact this stable vector by removing deleted elements,
    /// use the method [`make_compact`][StableVecFacade::make_compact] or
    /// [`reordering_make_compact`][StableVecFacade::reordering_make_compact]
    /// instead.
    pub fn shrink_to_fit(&mut self) {
        // `realloc` has the following preconditions:
        // - (a) `new_cap ≥ self.len()`
        // - (b) `new_cap ≤ isize::MAX`
        //
        // It's trivial to see that (a) is not violated here. (b) is also never
        // violated, because the `Core` trait says that `len < cap` and `cap <
        // isize::MAX`.
        unsafe {
            let new_cap = self.core.len();
            self.core.realloc(new_cap);
        }
    }

    /// Rearranges elements to reclaim memory. **Invalidates indices!**
    ///
    /// After calling this method, all existing elements stored contiguously in
    /// memory. You might want to call [`shrink_to_fit()`][StableVecFacade::shrink_to_fit]
    /// afterwards to actually free memory previously used by removed elements.
    /// This method itself does not deallocate any memory.
    ///
    /// The `next_push_index` value is also changed by this method (if the
    /// stable vector wasn't compact before).
    ///
    /// In comparison to
    /// [`reordering_make_compact()`][StableVecFacade::reordering_make_compact],
    /// this method does not change the order of elements. Due to this, this
    /// method is a bit slower.
    ///
    /// # Warning
    ///
    /// This method invalidates the indices of all elements that are stored
    /// after the first empty slot in the stable vector!
    pub fn make_compact(&mut self) {
        if self.is_compact() {
            return;
        }

        // We only have to move elements, if we have any.
        if self.num_elements > 0 {
            unsafe {
                // We have to find the position of the first hole. We know that
                // there is at least one hole, so we can unwrap.
                let first_hole_index = self.core.next_hole_from(0).unwrap();

                // This variable will store the first possible index of an element
                // which can be inserted in the hole.
                let mut element_index = first_hole_index + 1;

                // Beginning from the first hole, we have to fill each index with
                // a new value. This is required to keep the order of elements.
                for hole_index in first_hole_index..self.num_elements {
                    // Actually find the next element which we can use to fill the
                    // hole. Note that we do not check if `element_index` runs out
                    // of bounds. This will never happen! We do have enough
                    // elements to fill all holes. And once all holes are filled,
                    // the outer loop will stop.
                    while !self.core.has_element_at(element_index) {
                        element_index += 1;
                    }

                    // So at this point `hole_index` points to a valid hole and
                    // `element_index` points to a valid element. Time to swap!
                    self.core.swap(hole_index, element_index);
                }
            }
        }

        // We can safely call `set_len()` here: all elements are in the
        // range 0..self.num_elements.
        unsafe {
            self.core.set_len(self.num_elements);
        }
    }

    /// Rearranges elements to reclaim memory. **Invalidates indices and
    /// changes the order of the elements!**
    ///
    /// After calling this method, all existing elements stored contiguously
    /// in memory. You might want to call [`shrink_to_fit()`][StableVecFacade::shrink_to_fit]
    /// afterwards to actually free memory previously used by removed elements.
    /// This method itself does not deallocate any memory.
    ///
    /// The `next_push_index` value is also changed by this method (if the
    /// stable vector wasn't compact before).
    ///
    /// If you do need to preserve the order of elements, use
    /// [`make_compact()`][StableVecFacade::make_compact] instead. However, if
    /// you don't care about element order, you should prefer using this
    /// method, because it is faster.
    ///
    /// # Warning
    ///
    /// This method invalidates the indices of all elements that are stored
    /// after the first hole and it does not preserve the order of elements!
    pub fn reordering_make_compact(&mut self) {
        if self.is_compact() {
            return;
        }

        // We only have to move elements, if we have any.
        if self.num_elements > 0 {
            unsafe {
                // We use two indices:
                //
                // - `hole_index` starts from the front and searches for a hole
                //   that can be filled with an element.
                // - `element_index` starts from the back and searches for an
                //   element.
                let len = self.core.len();
                let mut element_index = len - 1;
                let mut hole_index = 0;
                loop {
                    element_index = self.core.prev_index_from(element_index).unwrap_or(0);
                    hole_index = self.core.next_hole_from(hole_index).unwrap_or(len);

                    // If both indices passed each other, we can stop. There are no
                    // holes left of `hole_index` and no element right of
                    // `element_index`.
                    if hole_index > element_index {
                        break;
                    }

                    // We found an element and a hole left of the element. That
                    // means that we can swap.
                    self.core.swap(hole_index, element_index);
                }
            }
        }

        // We can safely call `set_len()` here: all elements are in the
        // range 0..self.num_elements.
        unsafe {
            self.core.set_len(self.num_elements);
        }
    }

    /// Returns `true` if all existing elements are stored contiguously from
    /// the beginning (in other words: there are no empty slots with indices
    /// below `self.next_push_index()`).
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[0, 1, 2, 3, 4]);
    /// assert!(sv.is_compact());
    ///
    /// sv.remove(1);
    /// assert!(!sv.is_compact());
    /// ```
    pub fn is_compact(&self) -> bool {
        self.num_elements == self.core.len()
    }

    /// Returns the number of existing elements in this collection.
    ///
    /// As long as no element is ever removed, `num_elements()` equals
    /// `next_push_index()`. Once an element has been removed, `num_elements()`
    /// will always be less than `next_push_index()` (assuming
    /// `[reordering_]make_compact()` is not called).
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::new();
    /// assert_eq!(sv.num_elements(), 0);
    ///
    /// let heart_idx = sv.push('♥');
    /// assert_eq!(sv.num_elements(), 1);
    ///
    /// sv.remove(heart_idx);
    /// assert_eq!(sv.num_elements(), 0);
    /// ```
    pub fn num_elements(&self) -> usize {
        self.num_elements
    }

    /// Returns `true` if this collection doesn't contain any existing
    /// elements.
    ///
    /// This means that `is_empty()` returns true iff no elements were inserted
    /// *or* all inserted elements were removed again.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::new();
    /// assert!(sv.is_empty());
    ///
    /// let heart_idx = sv.push('♥');
    /// assert!(!sv.is_empty());
    ///
    /// sv.remove(heart_idx);
    /// assert!(sv.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.num_elements == 0
    }

    /// Removes all elements from this collection.
    ///
    /// After calling this, `num_elements()` will return 0. All indices are
    /// invalidated. However, no memory is deallocated, so the capacity stays
    /// as it was before. `self.next_push_index` is 0 after calling this method.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&['a', 'b']);
    ///
    /// sv.clear();
    /// assert_eq!(sv.num_elements(), 0);
    /// assert!(sv.capacity() >= 2);
    /// ```
    pub fn clear(&mut self) {
        self.core.clear();
        self.num_elements = 0;
    }

    /// Returns the number of slots in this stable vector.
    pub fn capacity(&self) -> usize {
        self.core.cap()
    }

    /// Returns the index that would be returned by calling
    /// [`push()`][StableVecFacade::push]. All filled slots have indices below
    /// `next_push_index()`.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&['a', 'b', 'c']);
    ///
    /// let next_push_index = sv.next_push_index();
    /// let index_of_d = sv.push('d');
    ///
    /// assert_eq!(next_push_index, index_of_d);
    /// ```
    pub fn next_push_index(&self) -> usize {
        self.core.len()
    }

    /// Returns the index of the next filled slot with index `idx` or higher.
    ///
    /// Specifically, if an element at index `idx` exists, `Some(idx)` is
    /// returned. If all slots with indices `idx` and higher are empty (or
    /// don't exist), `None` is returned. This method can be used to iterate
    /// over all existing elements without an iterator object.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[0, 1, 2, 3, 4]);
    /// sv.remove(1);
    /// sv.remove(2);
    /// sv.remove(4);
    ///
    /// assert_eq!(sv.next_index_from(0), Some(0));
    /// assert_eq!(sv.next_index_from(1), Some(3));
    /// assert_eq!(sv.next_index_from(2), Some(3));
    /// assert_eq!(sv.next_index_from(3), Some(3));
    /// assert_eq!(sv.next_index_from(4), None);
    /// assert_eq!(sv.next_index_from(5), None);
    /// ```
    pub fn next_index_from(&self, start: usize) -> Option<usize> {
        if start >= self.next_push_index() {
            None
        } else {
            // The precondition `start <= self.core.len()` is satisfied.
            unsafe { self.core.next_index_from(start) }
        }
    }

    /// Returns the index of the previous filled slot with index `idx` or
    /// lower. This is like `next_index_from` but searching backwards.
    ///
    /// Specifically, if an element at index `idx` exists, `Some(idx)` is
    /// returned. If all slots with indices `idx` and lower are empty, `None`
    /// is returned. This method can be used to iterate over all existing
    /// elements without an iterator object.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[0, 1, 2, 3, 4]);
    /// sv.remove(0);
    /// sv.remove(2);
    /// sv.remove(3);
    ///
    /// assert_eq!(sv.prev_index_from(0), None);
    /// assert_eq!(sv.prev_index_from(1), Some(1));
    /// assert_eq!(sv.prev_index_from(2), Some(1));
    /// assert_eq!(sv.prev_index_from(3), Some(1));
    /// assert_eq!(sv.prev_index_from(4), Some(4));
    /// assert_eq!(sv.prev_index_from(5), Some(4));
    /// ```
    pub fn prev_index_from(&self, start: usize) -> Option<usize> {
        // The precondition `start < self.core.len()` is satisfied de to this
        // `min` expression.
        let len = self.next_push_index();
        if len == 0 {
            return None;
        }

        let start = std::cmp::min(start, len - 1);
        unsafe { self.core.prev_index_from(start) }
    }

    /// Returns an iterator over immutable references to the existing elements
    /// of this stable vector. Elements are yielded in order of their
    /// increasing indices.
    ///
    /// Note that you can also use the `IntoIterator` implementation of
    /// `&StableVecFacade` to obtain the same iterator.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[0, 1, 2, 3, 4]);
    /// sv.remove(1);
    ///
    /// // Using the `iter()` method to apply a `filter()`.
    /// let mut it = sv.iter().filter(|&&n| n <= 3);
    /// assert_eq!(it.next(), Some(&0));
    /// assert_eq!(it.next(), Some(&2));
    /// assert_eq!(it.next(), Some(&3));
    /// assert_eq!(it.next(), None);
    ///
    /// // Simple iterate using the implicit `IntoIterator` conversion of the
    /// // for-loop:
    /// for e in &sv {
    ///     println!("{:?}", e);
    /// }
    /// ```
    pub fn iter(&self) -> Iter<'_, T, C> {
        Iter {
            core: &self.core,
            pos: 0,
            count: self.num_elements,
        }
    }

    /// Returns an iterator over mutable references to the existing elements
    /// of this stable vector. Elements are yielded in order of their
    /// increasing indices.
    ///
    /// Note that you can also use the `IntoIterator` implementation of
    /// `&mut StableVecFacade` to obtain the same iterator.
    ///
    /// Through this iterator, the elements within the stable vector can be
    /// mutated.
    ///
    /// # Examples
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[1.0, 2.0, 3.0]);
    ///
    /// for e in &mut sv {
    ///     *e *= 2.0;
    /// }
    ///
    /// assert_eq!(sv, &[2.0, 4.0, 6.0] as &[_]);
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<T, C> {
        IterMut {
            count: self.num_elements,
            sv: self,
            pos: 0,
        }
    }

    /// Returns an iterator over all indices of filled slots of this stable
    /// vector. Indices are yielded in increasing order.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&['a', 'b', 'c', 'd']);
    /// sv.remove(1);
    ///
    /// let mut it = sv.indices();
    /// assert_eq!(it.next(), Some(0));
    /// assert_eq!(it.next(), Some(2));
    /// assert_eq!(it.next(), Some(3));
    /// assert_eq!(it.next(), None);
    /// ```
    ///
    /// Simply using the `for`-loop:
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&['a', 'b', 'c', 'd']);
    ///
    /// for index in sv.indices() {
    ///     println!("index: {}", index);
    /// }
    /// ```
    pub fn indices(&self) -> Indices<'_, T, C> {
        Indices {
            core: &self.core,
            pos: 0,
            count: self.num_elements,
        }
    }

    /// Returns `true` if the stable vector contains an element with the given
    /// value, `false` otherwise.
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&['a', 'b', 'c']);
    /// assert!(sv.contains(&'b'));
    ///
    /// sv.remove(1);   // 'b' is stored at index 1
    /// assert!(!sv.contains(&'b'));
    /// ```
    pub fn contains<U>(&self, item: &U) -> bool
    where
        U: PartialEq<T>,
    {
        self.iter().any(|e| item == e)
    }

    /// Retains only the elements specified by the given predicate.
    ///
    /// Each element `e` for which `should_be_kept(&e)` returns `false` is
    /// removed from the stable vector.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[1, 2, 3, 4, 5]);
    /// sv.retain(|&e| e % 2 == 0);
    ///
    /// assert_eq!(sv, &[2, 4] as &[_]);
    /// ```
    pub fn retain<P>(&mut self, mut should_be_kept: P)
    where
        P: FnMut(&T) -> bool,
    {
        let mut pos = 0;

        // These unsafe calls are fine: indices returned by `next_index_from`
        // are always valid and point to an existing element.
        unsafe {
            while let Some(idx) = self.core.next_index_from(pos) {
                let elem = self.core.get_unchecked(idx);
                if !should_be_kept(elem) {
                    self.core.remove_at(idx);
                    self.num_elements -= 1;
                }

                pos = idx + 1;
            }
        }
    }

    /// Retains only the elements with indices specified by the given
    /// predicate.
    ///
    /// Each element with index `i` for which `should_be_kept(i)` returns
    /// `false` is removed from the stable vector.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::new();
    /// sv.push(1);
    /// let two = sv.push(2);
    /// sv.push(3);
    /// sv.retain_indices(|i| i == two);
    ///
    /// assert_eq!(sv, &[2] as &[_]);
    /// ```
    pub fn retain_indices<P>(&mut self, mut should_be_kept: P)
    where
        P: FnMut(usize) -> bool,
    {
        let mut pos = 0;

        // These unsafe call is fine: indices returned by
        // `next_index_from` are always valid and point to an existing
        // element.
        unsafe {
            while let Some(idx) = self.core.next_index_from(pos) {
                if !should_be_kept(idx) {
                    self.core.remove_at(idx);
                    self.num_elements -= 1;
                }

                pos = idx + 1;
            }
        }
    }

    /// Appends all elements in `new_elements` to this stable vector. This is
    /// equivalent to calling [`push()`][StableVecFacade::push] for each
    /// element.
    pub fn extend_from_slice(&mut self, new_elements: &[T])
    where
        T: Clone,
    {
        let len = new_elements.len();

        self.reserve(len);
        self.num_elements += len;

        // It's important that a panic in `clone()` does not lead to memory
        // unsafety! The only way that could happen is if some uninitialized
        // values would be read when `out` is dropped. However, this won't
        // happen: the core won't ever drop uninitialized elements.
        //
        // So that's good. But we also would like to drop all elements that
        // have already been inserted. That's why we set the length first.
        unsafe {
            let mut i = self.core.len();
            let new_len = self.core.len() + len;
            self.core.set_len(new_len);

            for elem in new_elements {
                self.core.insert_at(i, elem.clone());
                i += 1;
            }
        }
    }
}


#[inline(never)]
#[cold]
fn index_fail(idx: usize) -> ! {
    panic!("attempt to index StableVec with index {}, but no element exists at that index", idx);
}

impl<T, C: Core<T>> Index<usize> for StableVecFacade<T, C> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        match self.get(index) {
            Some(v) => v,
            None => index_fail(index),
        }
    }
}

impl<T, C: Core<T>> IndexMut<usize> for StableVecFacade<T, C> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        match self.get_mut(index) {
            Some(v) => v,
            None => index_fail(index),
        }
    }
}

impl<T, C: Core<T>> Default for StableVecFacade<T, C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, S, C: Core<T>> From<S> for StableVecFacade<T, C>
where
    S: AsRef<[T]>,
    T: Clone,
{
    fn from(slice: S) -> Self {
        let mut out = Self::new();
        out.extend_from_slice(slice.as_ref());
        out
    }
}

impl<T, C: Core<T>> FromIterator<T> for StableVecFacade<T, C> {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        let mut out = Self::new();
        out.extend(iter);
        out
    }
}

impl<T, C: Core<T>> Extend<T> for StableVecFacade<T, C> {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        let it = iter.into_iter();
        self.reserve(it.size_hint().0);

        for elem in it {
            self.push(elem);
        }
    }
}

/// Write into `StableVecFacade<u8>` by appending `u8` elements. This is
/// equivalent to calling `push` for each byte.
impl<C: Core<u8>> io::Write for StableVecFacade<u8, C> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.extend_from_slice(buf);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl<'a, T, C: Core<T>> IntoIterator for &'a StableVecFacade<T, C> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T, C>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, C: Core<T>> IntoIterator for &'a mut StableVecFacade<T, C> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T, C>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

impl<T, C: Core<T>> IntoIterator for StableVecFacade<T, C> {
    type Item = T;
    type IntoIter = IntoIter<T, C>;
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            sv: self,
            pos: 0,
        }
    }
}

impl<T: fmt::Debug, C: Core<T>> fmt::Debug for StableVecFacade<T, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StableVec ")?;
        f.debug_list().entries(self).finish()
    }
}

impl<Ta, Tb, Ca, Cb> PartialEq<StableVecFacade<Tb, Cb>> for StableVecFacade<Ta, Ca>
where
    Ta: PartialEq<Tb>,
    Ca: Core<Ta>,
    Cb: Core<Tb>,
{
    fn eq(&self, other: &StableVecFacade<Tb, Cb>) -> bool {
        self.iter().eq(other)
    }
}

impl<A, B, C: Core<A>> PartialEq<[B]> for StableVecFacade<A, C>
where
    A: PartialEq<B>,
{
    fn eq(&self, other: &[B]) -> bool {
        self.iter().eq(other)
    }
}

impl<'other, A, B, C: Core<A>> PartialEq<&'other [B]> for StableVecFacade<A, C>
where
    A: PartialEq<B>,
{
    fn eq(&self, other: &&'other [B]) -> bool {
        self == *other
    }
}

impl<A, B, C: Core<A>> PartialEq<Vec<B>> for StableVecFacade<A, C>
where
    A: PartialEq<B>,
{
    fn eq(&self, other: &Vec<B>) -> bool {
        self == &other[..]
    }
}
