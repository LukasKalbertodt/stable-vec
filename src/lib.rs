//! A `Vec<T>`-like collection which guarantees stable indices and features
//! O(1) deletion of elements.
//!
//! This crate provides a simple stable vector implementation. You can find
//! nearly all the relevant documentation on
//! [this crate's only type: `StableVec`](struct.StableVec.html).
//!
//! ---
//!
//! In order to use this crate, you have to include it into your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! stable_vec = "0.1"
//! ```
//!
//! ... as well as declare it at your crate root:
//!
//! ```ignore
//! extern crate stable_vec;
//!
//! use stable_vec::StableVec;
//! ```

extern crate bit_vec;
#[cfg(test)]
#[macro_use]
extern crate quickcheck;

use bit_vec::BitVec;

use std::ops::{Index, IndexMut};
use std::ptr;

#[cfg(test)]
mod tests;


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
///
/// # Note
///
/// This type's interface is very similar to the `Vec<T>` interface
/// from the Rust standard library. When in doubt about what a method is doing,
/// please consult [the official `Vec<T>` documentation][vec-doc] first.
///
/// [vec-doc]: https://doc.rust-lang.org/stable/std/vec/struct.Vec.html
#[derive(Clone, PartialEq, Eq)]
pub struct StableVec<T> {
    /// Storing the actual data.
    data: Vec<T>,

    /// A flag for each element saying whether the element was removed.
    deleted: BitVec,

    /// A cached value equal to `self.deleted.iter().filter(|&b| !b).count()`
    used_count: usize,
}

impl<T> StableVec<T> {
    /// Constructs a new, empty `StableVec<T>`.
    ///
    /// The stable-vector will not allocate until elements are pushed onto it.
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            deleted: BitVec::new(),
            used_count: 0,
        }
    }

    /// Constructs a new, empty `StableVec<T>` with the specified capacity.
    ///
    /// The stable-vector will be able to hold exactly `capacity` elements
    /// without reallocating. If `capacity` is 0, the stable-vector will not
    /// allocate any memory.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            deleted: BitVec::with_capacity(capacity),
            used_count: 0,
        }
    }

    /// Reserves capacity for at least `additional` more elements to be
    /// inserted.
    pub fn reserve(&mut self, additional: usize) {
        self.data.reserve(additional);
        self.deleted.reserve(additional);
    }

    /// Appends a new element to the back of the collection and returns the
    /// index of the inserted element.
    ///
    /// The inserted element will always be accessable via the returned index.
    pub fn push(&mut self, elem: T) -> usize {
        self.data.push(elem);
        self.deleted.push(false);
        self.used_count += 1;
        self.data.len() - 1
    }

    /// Removes and returns the last element from this collection, or `None` if
    /// it's empty.
    ///
    /// This method uses exactly the same deletion strategy as `remove()`.
    pub fn pop(&mut self) -> Option<T> {
        let last_index = self.deleted.iter()
            .enumerate()
            .rev()
            .find(|&(_, deleted)| !deleted)
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.remove(last_index)
    }

    /// Removes and returns the element at position `index` if the there
    /// `exists()` an element at that index.
    ///
    /// Removing an element only marks it as "deleted" without touching the
    /// actual data. In particular, the elements after the given index are
    /// **not+* shifted to the left. Thus, the time complexity of this method
    /// is O(1).
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index < self.data.len() && !self.deleted[index] {
            let elem = unsafe {
                self.deleted.set(index, true);
                ptr::read(&self.data[index])
            };
            self.used_count -= 1;
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
        if self.exists(index) {
            Some(&self.data[index])
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
        if self.exists(index) {
            Some(&mut self.data[index])
        } else {
            None
        }
    }

    /// Returns `true` if there exists an element at the given index, `false`
    /// otherwise.
    ///
    /// An element is said to exist if the index is not out of bounds and the
    /// element at the given index was not removed yet.
    pub fn exists(&self, index: usize) -> bool {
        index < self.data.len() && !self.deleted[index]
    }

    /// Calls `shrink_to_fit()` on the underlying `Vec<T>`.
    ///
    /// Note that this does not move existing elements around and thus does
    /// not invalidate indices. It only calls `shrink_to_fit()` on the
    /// `Vec<T>` that holds the actual data.
    ///
    /// If you want to compact this `StableVec` by removing deleted elements,
    /// use the method `compact()` instead.
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }

    /// Rearranges elements to reclaim memory. **Invalidates indices!**
    ///
    /// After calling this method, all existing elements stored contiguously
    /// in memory. You might want to call `shrink_to_fit()` afterwards to
    /// actually free memory previously used by removed elements. This method
    /// itself does not deallocate any memory.
    ///
    /// # Warning
    ///
    /// This method invalidates all indices! It does not even preserve the
    /// order of elements.
    pub fn compact(&mut self) {
        if self.is_compact() {
            return;
        }

        // We only have to move elements, if we have any.
        if self.used_count > 0 {
            // We use two indices:
            //
            // - `hole_index` starts from the front and searches for a hole that
            //   can be filled with an element.
            // - `element_index` starts from the back and searches for an element.
            //
            let len = self.data.len();
            let mut element_index = len - 1;
            let mut hole_index = 0;
            loop {
                // Advance `element_index` until we found an element.
                while element_index > 0 && self.deleted[element_index] {
                    element_index -= 1;
                }

                // Advance `hole_index` until we found a hole.
                while hole_index < len && !self.deleted[hole_index] {
                    hole_index += 1;
                }

                // If both indices passed each other, we can stop. There are no
                // holes left of `hole_index` and no element right of
                // `element_index`.
                if hole_index > element_index {
                    break;
                }

                /// We found an element and a hole left of the element. That means
                /// that we can swap.
                self.data.swap(hole_index, element_index);
                self.deleted.set(hole_index, false);
                self.deleted.set(element_index, true);
            }
        }

        // We can safely call `set_len()` here: all elements that still need
        // to be dropped are in the range 0..self.used_count + 1.
        unsafe {
            self.data.set_len(self.used_count);
            self.deleted.set_len(self.used_count);
        }
    }

    /// Returns `true` if all existing elements are stored contiguously from
    /// the beginning.
    ///
    /// This method returning `true` means that no memory is wasted for removed
    /// elements.
    pub fn is_compact(&self) -> bool {
        self.used_count == self.data.len()
    }

    /// Returns the number of existing elements in this collection.
    ///
    /// As long as `remove()` is never called, `num_elements()` equals
    /// `next_index()`. Once it is called, `num_elements()` will always be less
    /// than `next_index()` (assuming `compact()` is not called).
    pub fn num_elements(&self) -> usize {
        self.used_count
    }

    /// Returns `true` if this collection doesn't contain any existing
    /// elements.
    ///
    /// This means that `is_empty()` returns true iff no elements were inserted
    /// *or* all inserted elements were removed again.
    pub fn is_empty(&self) -> bool {
        self.used_count == 0
    }

    /// Returns the number of elements the stable-vector can hold without
    /// reallocating.
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Returns the index that would be returned by calling `push()`.
    pub fn next_index(&self) -> usize {
        self.data.len()
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            sv: self,
            pos: 0,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            deleted: &self.deleted,
            vec_iter: self.data.iter_mut(),
            pos: 0,
        }
    }
}

impl<T> Drop for StableVec<T> {
    fn drop(&mut self) {
        // We need to drop all elements that have not been removed. We can't
        // just run Vec's drop impl for `self.data` because this would attempt
        // to drop already dropped values. However, the Vec still needs to
        // free its memory.
        //
        // To achieve all this, we manually drop all remaining elements, then
        // tell the Vec that its length is 0 (its capacity stays the same!) and
        // let the Vec drop itself in the end.
        let living_indices = self.deleted.iter()
            .enumerate()
            .filter_map(|(i, deleted)| if deleted { None } else { Some(i) });
        for i in living_indices {
            unsafe {
                ptr::drop_in_place(&mut self.data[i]);
            }
        }

        unsafe {
            self.data.set_len(0);
        }
    }
}

impl<T> Index<usize> for StableVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        assert!(self.exists(index));

        &self.data[index]
    }
}

impl<T> IndexMut<usize> for StableVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        assert!(self.exists(index));

        &mut self.data[index]
    }
}

impl<T, S> From<S> for StableVec<T>
    where S: AsRef<[T]>,
          T: Clone
{
    fn from(slice: S) -> Self {
        let len = slice.as_ref().len();
        Self {
            data: slice.as_ref().into(),
            deleted: BitVec::from_elem(len, false),
            used_count: len,
        }
    }
}

impl<'a, T> IntoIterator for &'a StableVec<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut StableVec<T> {
    type Item = &'a mut T;
    type IntoIter = IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

pub struct Iter<'a, T: 'a> {
    sv: &'a StableVec<T>,
    pos: usize,
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.sv.data.len() {
            None
        } else {
            while self.pos < self.sv.deleted.len() && self.sv.deleted[self.pos] {
                self.pos += 1;
            }
            self.pos += 1;

            Some(&self.sv.data[self.pos - 1])
        }
    }
}

pub struct IterMut<'a, T: 'a> {
    deleted: &'a BitVec,
    vec_iter: ::std::slice::IterMut<'a, T>,
    pos: usize,
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.deleted.len() {
            None
        } else {
            while self.pos < self.deleted.len() && self.deleted[self.pos] {
                self.pos += 1;
                self.vec_iter.next();
            }
            self.pos += 1;
            self.vec_iter.next()
        }
    }
}
