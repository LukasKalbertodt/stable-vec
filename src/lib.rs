//! A `Vec<T>`-like collection which guarantees stable indices and features
//! O(1) deletion of elements.
//!
//! This crate provides a simple stable vector implementation. You can find
//! nearly all the relevant documentation on
//! [the type `StableVec`](struct.StableVec.html).
//!
//! ---
//!
//! In order to use this crate, you have to include it into your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! stable_vec = "0.2"
//! ```
//!
//! ... as well as declare it at your crate root:
//!
//! ```ignore
//! extern crate stable_vec;
//!
//! use stable_vec::StableVec;
//! ```

#![deny(missing_debug_implementations)]


use std::{
    fmt,
    io,
    iter::FromIterator,
    ops::{Index, IndexMut},
};

#[cfg(test)]
mod tests;
mod core;

use self::core::OwningCore;
pub use self::core::{
    Core,
    option::OptionCore,
    bitvec::BitVecCore,
};


/// The default core implementation of the stable vector.
pub type DefaultCore<T> = BitVecCore<T>;

pub type StableVec<T> = StableVecFacade<T, DefaultCore<T>>;
pub type InlineStableVec<T> = StableVecFacade<T, OptionCore<T>>;
pub type ExternStableVec<T> = StableVecFacade<T, BitVecCore<T>>;

/// A `Vec<T>`-like collection which guarantees stable indices and features
/// O(1) deletion of elements.
///
/// # Why?
///
/// The standard `Vec<T>` always stores all elements contiguous. While this has
/// many advantages (most notable: cache friendliness), it has the disadvantage
/// that you can't simply remove an element from the middle; at least not
/// without shifting all elements after it to the left. And this has two major
/// drawbacks:
///
/// 1. It has a linear O(n) time complexity
/// 2. It invalidates all indices of the shifted elements
///
/// Invalidating an index means that a given index `i` who referred to an
/// element `a` before, now refers to another element `b`. On the contrary, a
/// *stable* index means, that the index always refers to the same element.
///
/// Stable indices are needed in quite a few situations. One example are
/// graph data structures (or complex data structures in general). Instead of
/// allocating heap memory for every node and edge, all nodes are stored in a
/// vector and all edges are stored in a vector. But how does the programmer
/// unambiguously refer to one specific node? A pointer is not possible due to
/// the reallocation strategy of most dynamically growing arrays (the pointer
/// itself is not *stable*). Thus, often the index is used.
///
/// But in order to use the index, it has to be stable. This is one example,
/// where this data structure comes into play.
///
///
/// # How?
///
/// Actually, the implementation of this stable vector is very simple. We can
/// trade O(1) deletions and stable indices for a higher memory consumption.
///
/// When `StableVec::remove()` is called, the element is just marked as
/// "deleted", but no element is actually touched. This has the very obvious
/// disadvantage that deleted objects just stay in memory and waste space. This
/// is also the most important thing to understand:
///
/// The memory requirement of this data structure is `O(|inserted elements|)`;
/// instead of `O(|inserted elements| - |removed elements|)`. The latter is the
/// memory requirement of normal `Vec<T>`. Thus, if deletions are far more
/// numerous than insertions in your situation, then this data structure is
/// probably not fitting your needs.
///
///
/// # Why not?
///
/// As mentioned above, this data structure is very simple and has many
/// disadvantages on its own. Here are some reason not to use it:
///
/// - You don't need stable indices or O(1) removal
/// - Your deletions significantly outnumber your insertions
/// - You want to choose your keys/indices
/// - Lookup times do not matter so much to you
///
/// Especially in the last two cases, you could consider using a `HashMap` with
/// integer keys, best paired with a fast hash function for small keys.
///
/// If you not only want stable indices, but stable pointers, you might want
/// to use something similar to a linked list. Although: think carefully about
/// your problem before using a linked list.
///
///
/// # Note
///
/// This type's interface is very similar to the `Vec<T>` interface
/// from the Rust standard library. When in doubt about what a method is doing,
/// please consult [the official `Vec<T>` documentation][vec-doc] first.
///
/// [vec-doc]: https://doc.rust-lang.org/stable/std/vec/struct.Vec.html
///
///
/// # Method overview
///
/// (*there are more methods than mentioned in this overview*)
///
/// **Associated functions**
///
/// - [`new()`](#method.new)
/// - [`with_capacity()`](#method.with_capacity())
///
/// **Adding and removing elements**
///
/// - [`push()`](#method.push)
/// - [`pop()`](#method.pop)
/// - [`remove()`](#method.remove)
///
/// **Accessing elements**
///
/// - [`get()`](#method.get) (returns `Option<&T>`)
/// - [the `[]` index operator](#impl-Index<usize>) (returns `&T`)
/// - [`get_mut()`](#method.get_mut) (returns `Option<&mut T>`)
/// - [the mutable `[]` index operator](#impl-IndexMut<usize>) (returns `&mut T`)
/// - [`remove()`](#method.remove) (returns `Option<T>`)
///
/// **Stable vector specific**
///
/// - [`has_element_at()`](#method.has_element_at)
/// - [`next_index()`](#method.next_index)
/// - [`is_compact()`](#method.is_compact)
/// - [`make_compact()`](#method.make_compact)
/// - [`reordering_make_compact()`](#method.reordering_make_compact)
///
/// **Number of elements**
///
/// - [`is_empty()`](#method.is_empty)
/// - [`num_elements()`](#method.num_elements)
///
/// **Capacity management**
///
/// - [`capacity()`](#method.capacity)
/// - [`shrink_to_fit()`](#method.shrink_to_fit)
/// - [`reserve()`](#method.reserve)
///
// #[derive(PartialEq, Eq)]
#[derive(Clone)]
pub struct StableVecFacade<T, C: Core<T>> {
    core: OwningCore<T, C>,
    num_elements: usize,
}

impl<T, C: Core<T>> StableVecFacade<T, C> {
    /// Constructs a new, empty `StableVecFacade<T>`.
    ///
    /// The stable-vector will not allocate until elements are pushed onto it.
    pub fn new() -> Self {
        Self {
            core: OwningCore::new(C::new()),
            num_elements: 0,
        }
    }

    /// Constructs a new, empty `StableVecFacade<T>` with the specified capacity.
    ///
    /// The stable-vector will be able to hold exactly `capacity` elements
    /// without reallocating. If `capacity` is 0, the stable-vector will not
    /// allocate any memory.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut out = Self::new();
        out.reserve(capacity);
        out
    }

    /// Creates a `StableVecFacade<T>` from the given `Vec<T>`. The elements are not
    /// cloned (just moved) and the indices of the vector are preserved.
    ///
    /// Note that this function will still allocate memory.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from_vec(vec!['★', '♥']);
    ///
    /// assert_eq!(sv.get(0), Some(&'★'));
    /// assert_eq!(sv.get(1), Some(&'♥'));
    /// assert_eq!(sv.num_elements(), 2);
    /// assert!(sv.is_compact());
    ///
    /// sv.remove(0);
    /// assert_eq!(sv.get(1), Some(&'♥'));
    /// ```
    pub fn from_vec(vec: Vec<T>) -> Self {
        let mut core = C::new();
        let len = vec.len();

        unsafe {
            core.realloc(len);
            core.set_len(len);

            for (i, elem) in vec.into_iter().enumerate() {
                // Due to the `grow` above we know that `i` is always greater
                // than `core.capacity()`. And because we started with an empty
                // instance, all elements start out as deleted.
                core.insert_at(i, elem);
            }
        }

        Self {
            num_elements: len,
            core: OwningCore::new(core),
        }
    }

    /// Reserves capacity for at least `additional` more elements to be
    /// inserted.
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

    /// Appends a new element to the back of the collection and returns the
    /// index of the inserted element.
    ///
    /// The inserted element will always be accessible via the returned index.
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

    /// Removes and returns the first element from this collection, or `None` if
    /// it's empty.
    ///
    /// This method uses exactly the same deletion strategy as
    /// [`remove()`](#method.remove).
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[1, 2, 3]);
    /// assert_eq!(sv.remove_first(), Some(1));
    /// assert_eq!(sv.into_vec(), vec![2, 3]);
    /// ```
    ///
    /// # Note
    ///
    /// This method needs to find index of the first valid element. Finding it
    /// has a worst case time complexity of O(n). If you already know the
    /// index, use [`remove()`](#method.remove) instead.
    pub fn remove_first(&mut self) -> Option<T> {
        self.find_first_index().and_then(|index| self.remove(index))
    }

    /// Removes and returns the last element from this collection, or `None` if
    /// it's empty.
    ///
    /// This method uses exactly the same deletion strategy as
    /// [`remove()`](#method.remove).
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[1, 2, 3]);
    /// assert_eq!(sv.remove_last(), Some(3));
    /// assert_eq!(sv.into_vec(), vec![1, 2]);
    /// ```
    ///
    /// # Note
    ///
    /// This method needs to find index of the last valid element. Finding it
    /// has a worst case time complexity of O(n). If you already know the
    /// index, use [`remove()`](#method.remove) instead.
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

    /// Finds the first element and returns a mutable reference to it, or `None` if
    /// the stable vector is empty.
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
    /// assert_eq!(&sv.into_vec(), &[3, 2]);
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

    /// Finds the last element and returns a mutable reference to it, or `None` if
    /// the stable vector is empty.
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
    /// assert_eq!(&sv.into_vec(), &[1, 3]);
    /// ```
    pub fn find_last_mut(&mut self) -> Option<&mut T> {
        self.find_last_index().map(move |index| unsafe { self.core.get_unchecked_mut(index) })
    }

    /// Finds the first element and returns its index, or `None` if
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
    /// assert_eq!(sv.find_first_index(), Some(1));
    /// ```
    pub fn find_first_index(&self) -> Option<usize> {
        unsafe {
            self.core.next_index_from(0)
        }
    }

    /// Finds the last element and returns its index, or `None` if
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

    /// Inserts the given value at the given index if there is a hole there.
    ///
    /// If there is an element marked as "deleted" at `index`, the `elem` is
    /// inserted at that position and `Ok(())` is returned. If `index` is out of
    /// bounds or there is an existing element at that position, the vector is
    /// not changed and `elem` is returned as `Err(elem)`.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::new();
    /// let star_idx = sv.push('★');
    /// let heart_idx = sv.push('♥');
    ///
    /// // Inserting fails: there isn't a hole yet.
    /// assert_eq!(sv.insert_into_hole(star_idx, 'x'), Err('x'));
    /// assert_eq!(sv.num_elements(), 2);
    ///
    /// // After removing the star...
    /// sv.remove(star_idx);
    /// assert_eq!(sv.num_elements(), 1);
    ///
    /// // ...we can insert a new element at its place.
    /// assert_eq!(sv.insert_into_hole(star_idx, 'x'), Ok(()));
    /// assert_eq!(sv[star_idx], 'x');
    /// assert_eq!(sv.num_elements(), 2);
    /// ```
    pub fn insert_into_hole(&mut self, index: usize, elem: T) -> Result<(), T> {
        // If the index is out of bounds, we cannot insert the new element.
        if index >= self.core.len() {
            return Err(elem);
        }

        // We did the bounds check above, so this is fine.
        if unsafe { self.core.has_element_at(index) } {
            return Err(elem);
        }

        self.num_elements += 1;
        if self.core.len() <= index {
            // Due to the bounds check above, we know that `index + 1` is ≤
            // `capacity`.
            unsafe {
                self.core.set_len(index + 1);
            }
        }

        // We made sure of the two requirements above.
        unsafe {
            self.core.insert_at(index, elem);
        }

        Ok(())
    }

    /// Grows the size of the stable vector by inserting deleted elements.
    ///
    /// This method does not add existing elements, but merely "deleted" ones.
    /// Using this only makes sense when you are intending to use the holes
    /// with [`insert_into_hole()`](#method.insert_into_hole) later. Otherwise,
    /// this method will just waste memory.
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::new();
    /// let star_idx = sv.push('★');
    ///
    /// // After we inserted one element, the next element sits at index 1, as
    /// // expected.
    /// assert_eq!(sv.next_index(), 1);
    ///
    /// sv.grow(2); // insert two deleted elements
    ///
    /// assert_eq!(sv.num_elements(), 1); // Still only one existing element
    /// assert_eq!(sv.next_index(), 3); // Due to grow(2), we skip two indices
    ///
    /// // Now we can insert an element at index 2.
    /// sv.insert_into_hole(2, 'x').unwrap();
    /// assert_eq!(sv.num_elements(), 2);
    /// ```
    pub fn grow(&mut self, additional: usize) {
        let new_len = self.core.len() + additional;
        self.core.reserve(additional);

        unsafe {
            self.core.set_len(new_len);
        }
    }

    /// Removes and returns the element at position `index` if there exists an
    /// element at that index (as defined by
    /// [`has_element_at()`](#method.has_element_at)).
    ///
    /// Removing an element only marks it as "deleted" without touching the
    /// actual data. In particular, the elements after the given index are
    /// **not** shifted to the left. Thus, the time complexity of this method
    /// is O(1).
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

    /// Returns `true` if there exists an element at the given index, `false`
    /// otherwise.
    ///
    /// An element is said to exist if the index is not out of bounds and the
    /// element at the given index was not removed yet.
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
        if self.core.cap() <= index {
            return false;
        } else {
            unsafe {
                // The index is smaller than the capacity, as checked aboved,
                // so we can call this without a problem.
                self.core.has_element_at(index)
            }
        }
    }

    /// Calls `shrink_to_fit()` on the underlying `Vec<T>`.
    ///
    /// Note that this does not move existing elements around and thus does not
    /// invalidate indices. This method also doesn't change what `next_index`
    /// returns. Instead, only capacity is changed; specifically, it is equal
    /// to `next_index` after calling this method.
    ///
    /// If you want to compact this stable vector by removing deleted elements,
    /// use the method [`make_compact`] or [`reordering_make_compact`] instead.
    pub fn shrink_to_fit(&mut self) {
        let new_cap = self.core.len();
        unsafe {
            self.core.realloc(new_cap);
        }
    }

    /// Rearranges elements to reclaim memory. **Invalidates indices!**
    ///
    /// After calling this method, all existing elements stored contiguously
    /// in memory. You might want to call [`shrink_to_fit()`](#method.shrink_to_fit)
    /// afterwards to actually free memory previously used by removed elements.
    /// This method itself does not deallocate any memory.
    ///
    /// In comparison to
    /// [`reordering_make_compact()`](#method.reordering_make_compact), this
    /// method does not change the order of elements. Due to this, this method
    /// is a bit slower.
    ///
    /// # Warning
    ///
    /// This method invalidates the indices of all elements that are stored
    /// after the first hole in the stable vector!
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
    /// in memory. You might want to call [`shrink_to_fit()`](#method.shrink_to_fit)
    /// afterwards to actually free memory previously used by removed elements.
    /// This method itself does not deallocate any memory.
    ///
    /// If you do need to preserve the order of elements, use
    /// [`make_compact()`](#method.make_compact) instead. However, if you don't
    /// care about element order, you should prefer using this method, because
    /// it is faster.
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
    /// the beginning.
    ///
    /// This method returning `true` means that no memory is wasted for removed
    /// elements.
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
    /// As long as `remove()` is never called, `num_elements()` equals
    /// `next_index()`. Once it is called, `num_elements()` will always be less
    /// than `next_index()` (assuming `make_compact()` is not called).
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
    /// as it was before.
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

    /// Returns the number of elements the stable-vector can hold without
    /// reallocating.
    pub fn capacity(&self) -> usize {
        self.core.cap()
    }

    /// Returns the index that would be returned by calling
    /// [`push()`](#method.push).
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&['a', 'b', 'c']);
    ///
    /// let next_index = sv.next_index();
    /// let index_of_d = sv.push('d');
    ///
    /// assert_eq!(next_index, index_of_d);
    /// ```
    pub fn next_index(&self) -> usize {
        self.core.len()
    }

    /// Returns an iterator over immutable references to the existing elements
    /// of this stable vector.
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
    /// of this stable vector.
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

    /// Returns an iterator over all valid indices of this stable vector.
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
        for e in self {
            if item == e {
                return true;
            }
        }
        false
    }

    /// Returns the stable vector as a standard `Vec<T>`.
    ///
    /// Returns a vector which contains all existing elements from this stable
    /// vector. **All indices might be invalidated!** This method is equivalent
    /// to `self.into_iter().colect()`.
    ///
    ///
    /// # Example
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&['a', 'b', 'c']);
    /// sv.remove(1);   // 'b' lives at index 1
    ///
    /// assert_eq!(sv.into_vec(), vec!['a', 'c']);
    /// ```
    pub fn into_vec(self) -> Vec<T> {
        // TODO: maybe improve performance in special case: if vector is
        // compact and core already stores a `Vec<T>`.
        self.into_iter().collect()
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

        while let Some(idx) = unsafe { self.core.next_index_from(pos) } {
            // These unsafe calls are fine: indices returned by
            // `next_index_from` are always valid and point to an existing
            // element.
            let elem = unsafe { self.core.get_unchecked(idx) };
            if !should_be_kept(elem) {
                unsafe {
                    self.core.remove_at(idx);
                }
                self.num_elements -= 1;
            }

            pos = idx + 1;
        }
    }

    /// Retains only the elements with indices specified by the given predicate.
    ///
    /// Each element with index `i` for which `should_be_kept(i)` returns `false` is
    /// removed from the stable vector.
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

        while let Some(idx) = unsafe { self.core.next_index_from(pos) } {
            if !should_be_kept(idx) {
                // These unsafe call is fine: indices returned by
                // `next_index_from` are always valid and point to an existing
                // element.
                unsafe {
                    self.core.remove_at(idx);
                }
                self.num_elements -= 1;
            }

            pos = idx + 1;
        }
    }

    /// Appends all elements in `new_elements` to this stable vector. This is
    /// equivalent to calling [`push()`][StableVecFacade::push] for each element.
    pub fn extend_from_slice(&mut self, new_elements: &[T])
    where
        T: Clone,
    {
        let len = new_elements.len();

        self.core.reserve(len);
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

/// Write into `StableVecFacade<u8>` by appending `u8` elements. This is equivalent
/// to calling `push` for each byte.
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


/// Iterator over immutable references to the elements of a `StableVecFacade`.
///
/// Use the method [`StableVecFacade::iter()`](struct.StableVecFacade.html#method.iter) or
/// the `IntoIterator` implementation of `&StableVecFacade` to obtain an iterator
/// of this kind.
pub struct Iter<'a, T, C: Core<T>> {
    core: &'a OwningCore<T, C>,
    pos: usize,
    count: usize,
}

impl<'a, T, C: Core<T>> Iterator for Iter<'a, T, C> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        let idx = unsafe { self.core.next_index_from(self.pos) };
        if let Some(idx) = idx {
            self.pos = idx + 1;
            self.count -= 1;
        }

        idx.map(|idx| unsafe { self.core.get_unchecked(idx) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}

impl<T, C: Core<T>> ExactSizeIterator for Iter<'_, T, C> {}

impl<T, C: Core<T>> fmt::Debug for Iter<'_, T, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Iter")
            .field("pos", &self.pos)
            .field("count", &self.count)
            .finish()
    }
}


/// Iterator over mutable references to the elements of a `StableVecFacade`.
///
/// Use the method [`StableVecFacade::iter_mut()`](struct.StableVecFacade.html#method.iter_mut)
/// or the `IntoIterator` implementation of `&mut StableVecFacade` to obtain an
/// iterator of this kind.
pub struct IterMut<'a, T, C: Core<T>> {
    sv: &'a mut StableVecFacade<T, C>,
    pos: usize,
    count: usize,
}

impl<'a, T, C: Core<T>> Iterator for IterMut<'a, T, C> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = unsafe { self.sv.core.next_index_from(self.pos) };
        if let Some(idx) = idx {
            self.pos = idx + 1;
            self.count -= 1;
        }

        // This is... scary. We are extending the lifetime of the reference
        // returned by `get_unchecked_mut`. We can do that because we know that
        // we will never return the same reference twice. So the user can't
        // have mutable aliases. Furthermore, all access to the original stable
        // vector is blocked because we (`IterMut`) have a mutable reference to
        // it. So it is fine to extend the lifetime to `'a`.
        idx.map(|idx| {
            unsafe { &mut *(self.sv.core.get_unchecked_mut(idx) as *mut T) }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}

impl<T, C: Core<T>> ExactSizeIterator for IterMut<'_, T, C> {}

impl<T, C: Core<T>> fmt::Debug for IterMut<'_, T, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("IterMut")
            .field("pos", &self.pos)
            .field("count", &self.count)
            .finish()
    }
}


/// Iterator over owned elements of a `StableVecFacade`.
///
/// Use the method [`StableVecFacade::into_iter`] to obtain an iterator of this kind.
#[derive(Debug)]
pub struct IntoIter<T, C: Core<T>> {
    sv: StableVecFacade<T, C>,
    pos: usize,
}

impl<T, C: Core<T>> Iterator for IntoIter<T, C> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = unsafe { self.sv.core.next_index_from(self.pos) };
        if let Some(idx) = idx {
            self.pos = idx + 1;
            self.sv.num_elements -= 1;
            let elem = unsafe {
                // We know that `idx` is a valid.
                self.sv.core.remove_at(idx)
            };

            Some(elem)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.sv.num_elements, Some(self.sv.num_elements))
    }
}

impl<T, C: Core<T>> ExactSizeIterator for IntoIter<T, C> {}


/// Iterator over all valid indices of a `StableVecFacade`.
///
/// Use the method [`StableVecFacade::indices`] to obtain an iterator of this kind.
pub struct Indices<'a, T, C: Core<T>> {
    core: &'a OwningCore<T, C>,
    pos: usize,
    count: usize,
}

impl<T, C: Core<T>> Iterator for Indices<'_, T, C> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        let out = unsafe { self.core.next_index_from(self.pos) };
        if let Some(idx) = out {
            self.pos = idx + 1;
            self.count -= 1;
        }

        out
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}

impl<T, C: Core<T>> ExactSizeIterator for Indices<'_, T, C> {}

impl<T, C: Core<T>> fmt::Debug for Indices<'_, T, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Indices")
            .field("pos", &self.pos)
            .field("count", &self.count)
            .finish()
    }
}


impl<T: fmt::Debug, C: Core<T>> fmt::Debug for StableVecFacade<T, C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StableVec ")?;
        f.debug_list().entries(self).finish()
    }
}

impl<Ta, Tb, Ca: Core<Ta>, Cb: Core<Tb>> PartialEq<StableVecFacade<Tb, Cb>> for StableVecFacade<Ta, Ca>
where
    Ta: PartialEq<Tb>,
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
