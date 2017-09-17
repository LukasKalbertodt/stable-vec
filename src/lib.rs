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

extern crate bit_vec;
#[cfg(test)]
#[macro_use]
extern crate quickcheck;

use bit_vec::BitVec;

use std::fmt;
use std::iter::FromIterator;
use std::mem;
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
        self.data.push(elem);
        self.deleted.push(false);
        self.used_count += 1;
        self.data.len() - 1
    }

    /// Removes and returns the last element from this collection, or `None` if
    /// it's empty.
    ///
    /// This method uses exactly the same deletion strategy as
    /// [`remove()`](#method.remove).
    ///
    /// # Note
    ///
    /// This method needs to find index of the last valid element. Finding it
    /// has a worst case time complexity of O(n). If you already know the
    /// index, use [`remove()`](#method.remove) instead.
    pub fn pop(&mut self) -> Option<T> {
        let last_index = self.deleted.iter()
            .enumerate()
            .rev()
            .find(|&(_, deleted)| !deleted)
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.remove(last_index)
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
            // We move the requested element out of our `data` vector. Usually,
            // it's impossible to move out of a vector without removing the
            // element in the vector. We can achieve it by using unsafe code:
            // We just read the value from the vector without changing
            // anything. This is dangerous if we try to access this element
            // in the vector later. To prevent any access, we mark the element
            // as deleted.
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
        if self.has_element_at(index) {
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
        if self.has_element_at(index) {
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
        index < self.data.len() && !self.deleted[index]
    }

    /// Calls `shrink_to_fit()` on the underlying `Vec<T>`.
    ///
    /// Note that this does not move existing elements around and thus does
    /// not invalidate indices. It only calls `shrink_to_fit()` on the
    /// `Vec<T>` that holds the actual data.
    ///
    /// If you want to compact this `StableVec` by removing deleted elements,
    /// use the method [`compact()`](#method.compact) instead.
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }

    /// Rearranges elements to reclaim memory. **Invalidates indices!**
    ///
    /// After calling this method, all existing elements stored contiguously
    /// in memory. You might want to call [`shrink_to_fit()`](#method.shrink_to_fit)
    /// afterwards to actually free memory previously used by removed elements.
    /// This method itself does not deallocate any memory.
    ///
    /// In comparison to [`compact()`](#method.compact), this method does not
    /// change the order of elements. Due to this, this method is a bit slower.
    ///
    /// # Warning
    ///
    /// This method invalidates all indices!
    pub fn make_compact(&mut self) {
        if self.is_compact() {
            return;
        }

        // We only have to move elements, if we have any.
        if self.used_count > 0 {
            // We have to find the position of the first hole. We know that
            // there is at least one hole, so we can unwrap.
            let first_hole_index = self.deleted.iter().position(|d| d).unwrap();

            // This variable will store the first possible index of an element
            // which can be inserted in the hole.
            let mut element_index = first_hole_index + 1;

            // Beginning from the first hole, we have to fill each index with
            // a new value. This is required to keep the order of elements.
            for hole_index in first_hole_index..self.used_count {
                // Actually find the next element which we can use to fill the
                // hole. Note that we do not check if `element_index` runs out
                // of bounds. This will never happen! We do have enough
                // elements to fill all holes. And once all holes are filled,
                // the outer loop will stop.
                while self.deleted[element_index] {
                    element_index += 1;
                }

                // So at this point `hole_index` points to a valid hole and
                // `element_index` points to a valid element. Time to swap!
                self.data.swap(hole_index, element_index);
                self.deleted.set(hole_index, false);
                self.deleted.set(element_index, true);
            }
        }

        // We can safely call `set_len()` here: all elements that still need
        // to be dropped are in the range 0..self.used_count.
        unsafe {
            self.data.set_len(self.used_count);
            self.deleted.set_len(self.used_count);
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
    /// This method invalidates all indices and it does not preserve the order
    /// of elements!
    pub fn reordering_make_compact(&mut self) {
        if self.is_compact() {
            return;
        }

        // We only have to move elements, if we have any.
        if self.used_count > 0 {
            // We use two indices:
            //
            // - `hole_index` starts from the front and searches for a hole
            //   that can be filled with an element.
            // - `element_index` starts from the back and searches for an
            //   element.
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

                // We found an element and a hole left of the element. That
                // means that we can swap.
                self.data.swap(hole_index, element_index);
                self.deleted.set(hole_index, false);
                self.deleted.set(element_index, true);
            }
        }

        // We can safely call `set_len()` here: all elements that still need
        // to be dropped are in the range 0..self.used_count.
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
        self.used_count == self.data.len()
    }

    /// Returns the number of existing elements in this collection.
    ///
    /// As long as `remove()` is never called, `num_elements()` equals
    /// `next_index()`. Once it is called, `num_elements()` will always be less
    /// than `next_index()` (assuming `compact()` is not called).
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
        self.used_count
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
        self.used_count == 0
    }

    /// Returns the number of elements the stable-vector can hold without
    /// reallocating.
    pub fn capacity(&self) -> usize {
        self.data.capacity()
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
        self.data.len()
    }

    /// Returns an iterator over immutable references to the existing elements
    /// of this stable vector.
    ///
    /// Note that you can also use the `IntoIterator` implementation of
    /// `&StableVec` to obtain the same iterator.
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
    pub fn iter(&self) -> Iter<T> {
        Iter {
            sv: self,
            pos: 0,
        }
    }

    /// Returns an iterator over mutable references to the existing elements
    /// of this stable vector.
    ///
    /// Note that you can also use the `IntoIterator` implementation of
    /// `&mut StableVec` to obtain the same iterator.
    ///
    /// Through this iterator, the elements within the stable vector can be
    /// mutated. Furthermore, you can remove elements from the stable vector
    /// during iteration by calling
    /// [`remove_current()`](struct.IterMut.html#method.remove_current) on the
    /// iterator object.
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
    ///
    /// As mentioned above, you can remove elements from the stable vector
    /// while iterating over it. But you can't use a `for`-loop in this case.
    ///
    /// ```
    /// # use stable_vec::StableVec;
    /// let mut sv = StableVec::from(&[1.0, 2.0, 3.0]);
    ///
    /// {
    ///     let mut it = sv.iter_mut();
    ///     while let Some(e) = it.next() {
    ///         if *e == 2.0 {
    ///             it.remove_current();
    ///         }
    ///         *e *= 2.0;
    ///     }
    /// }
    ///
    /// assert_eq!(sv, &[2.0, 6.0] as &[_]);
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            deleted: &mut self.deleted,
            used_count: &mut self.used_count,
            vec_iter: self.data.iter_mut(),
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
    /// let mut it = sv.keys();
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
    /// for index in sv.keys() {
    ///     println!("index: {}", index);
    /// }
    /// ```
    pub fn keys(&self) -> Keys {
        Keys {
            deleted: &self.deleted,
            pos: 0,
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
        where U: PartialEq<T>
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
    /// vector. **All indices might be invalidated!** This method might call
    /// [`make_compact()`](#method.make_compact); see that method's
    /// documentation to learn about the effects on indices.
    ///
    /// This method does not allocate memory.
    ///
    /// # Note
    ///
    /// If the stable vector is not compact (as defined by `is_compact()`), the
    /// runtime complexity of this function is O(n), because `make_compact()`
    /// needs to be called.
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
    pub fn into_vec(mut self) -> Vec<T> {
        // Compact the stable vec to prepare the `data` vector for moving.
        self.make_compact();

        // We reset all values to the "empty state" here. This is necessary to
        // make sure the `drop()` impl doesn't do anything (except for actually
        // freeing the memory of `deleted`).
        self.used_count = 0;
        self.deleted.truncate(0);

        // The `data` vector is moved out of this data structure and replaced
        // with an empty vector. After this line, `self` is dropped.
        mem::replace(&mut self.data, Vec::new())
    }

    /// Retains only the elements specified by the given predicate.
    ///
    /// Each element `e` for which `predicate(&e)` returns `false` is removed
    /// from the stable vector.
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
    pub fn retain<P>(&mut self, mut predicate: P)
        where P: FnMut(&T) -> bool,
    {
        let mut it = self.iter_mut();
        while let Some(e) = it.next() {
            if !predicate(e) {
                it.remove_current();
            }
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
        assert!(self.has_element_at(index));

        &self.data[index]
    }
}

impl<T> IndexMut<usize> for StableVec<T> {
    fn index_mut(&mut self, index: usize) -> &mut T {
        assert!(self.has_element_at(index));

        &mut self.data[index]
    }
}

impl<T> Default for StableVec<T> {
    fn default() -> Self {
        Self::new()
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

impl<T> FromIterator<T> for StableVec<T> {
    fn from_iter<I>(iter: I) -> Self
        where I: IntoIterator<Item = T>
    {
        let data = Vec::from_iter(iter);
        Self {
            used_count: data.len(),
            deleted: BitVec::from_elem(data.len(), false),
            data,
        }
    }
}

impl<T> Extend<T> for StableVec<T> {
    fn extend<I>(&mut self, iter: I)
        where I: IntoIterator<Item = T>
    {
        // This implementation is not completely exception safe. If the
        // `self.data.extend()` call panics, we won't drop any of the new
        // elements. This is "safe" in the Rust meaning of the word: not
        // calling `drop()` on values is not desireable but not considered
        // *unsafe*.
        let len_before = self.data.len();
        self.data.extend(iter);

        let additional_count = self.data.len() - len_before;
        self.deleted.grow(additional_count, false);
        self.used_count += additional_count;
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

/// Iterator over immutable references to the elements of a `StableVec`.
///
/// Use the method [`StableVec::iter()`](struct.StableVec.html#method.iter) or
/// the `IntoIterator` implementation of `&StableVec` to obtain an iterator
/// of this kind.
pub struct Iter<'a, T: 'a> {
    sv: &'a StableVec<T>,
    pos: usize,
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        next_valid_index(&mut self.pos, &self.sv.deleted)
            .map(|i| &self.sv.data[i])
    }
}

/// Iterator over mutable references to the elements of a `StableVec`.
///
/// Use the method [`StableVec::iter_mut()`](struct.StableVec.html#method.iter_mut)
/// or the `IntoIterator` implementation of `&mut StableVec` to obtain an
/// iterator of this kind.
pub struct IterMut<'a, T: 'a> {
    deleted: &'a mut BitVec,
    used_count: &'a mut usize,
    vec_iter: ::std::slice::IterMut<'a, T>,
    pos: usize,
}

impl<'a, T: 'a> IterMut<'a, T> {
    /// Removes the element that was returned by the last `next()` call from
    /// the underlying stable vector.
    ///
    /// # Panic
    ///
    /// This method panics if `next()` hasn't been called yet or if `next()`
    /// returned `None` the last time it was called.
    pub fn remove_current(&mut self) {
        assert!(self.pos != 0);

        self.deleted.set(self.pos - 1, true);
        *self.used_count -= 1;
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<Self::Item> {
        // First, we advance until we have found an existing element or until
        // we have reached the end of all elements.
        while self.pos < self.deleted.len() && self.deleted[self.pos] {
            self.pos += 1;
            self.vec_iter.next();
        }

        // Next, we check whether we are at the very end.
        if self.pos == self.deleted.len() {
            None
        } else {
            // Advance the iterator by one and return current element.
            self.pos += 1;
            self.vec_iter.next()
        }
    }
}

/// Iterator over all valid indices of a `StableVec`.
///
/// Use the method [`StableVec::keys()`](struct.StableVec.html#method.keys) to
/// obtain an iterator of this kind.
pub struct Keys<'a> {
    deleted: &'a BitVec,
    pos: usize,
}

impl<'a> Iterator for Keys<'a> {
    type Item = usize;
    fn next(&mut self) -> Option<Self::Item> {
        next_valid_index(&mut self.pos, self.deleted)
    }
}

/// Advances the index `pos` while it points to a deleted element. Stops
/// advancing once an existing element is found or the end is reached. In the
/// former case, this element's index is returned; in the latter case, `None`
/// is returned.
///
/// After this function was called, the value of `pos` is:
///
/// - `i + 1` if `Some(i)` was returned
/// - `deleted.len()` if `None` was returned
fn next_valid_index(pos: &mut usize, deleted: &BitVec) -> Option<usize> {
    // First, we advance until we have found an existing element or until
    // we have reached the end of all elements.
    while *pos < deleted.len() && deleted[*pos] {
        *pos += 1;
    }

    // Next, we check whether we are at the very end.
    if *pos == deleted.len() {
        None
    } else {
        // Advance by one and return current position.
        *pos += 1;
        Some(*pos - 1)
    }
}

impl<T: fmt::Debug> fmt::Debug for StableVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StableVec ")?;
        f.debug_list().entries(self).finish()
    }
}

impl<A, B> PartialEq<[B]> for StableVec<A>
    where A: PartialEq<B>,
{
    fn eq(&self, other: &[B]) -> bool {
        for (i, e) in self.iter().enumerate() {
            if e != &other[i] {
                return false;
            }
        }
        true
    }
}

impl<'other, A, B> PartialEq<&'other [B]> for StableVec<A>
    where A: PartialEq<B>,
{
    fn eq(&self, other: & &'other [B]) -> bool {
        self == *other
    }
}

impl<A, B> PartialEq<Vec<B>> for StableVec<A>
    where A: PartialEq<B>,
{
    fn eq(&self, other: &Vec<B>) -> bool {
        self == &other[..]
    }
}
