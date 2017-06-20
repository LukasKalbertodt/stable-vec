extern crate bit_vec;

use bit_vec::BitVec;

use std::ptr;


/// A `Vec<T>` like collection which guarantees stable indices.
///
///
/// *Note*: this type's interface is very similar to the `Vec<T>` interface
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
    ///
    /// *Note*: this method needs to find index of the last valid element.
    /// Finding it has a worst case time complexity of O(n). If you already
    /// know the index, use `remove()` instead.
    pub fn pop(&mut self) -> Option<T> {
        let last_index = self.deleted.iter()
            .enumerate()
            .rev()
            .find(|&(_, deleted)| !deleted)
            .map(|(i, _)| i)
            .unwrap_or(0);
        self.remove(last_index)
    }

    /// Removes and returns the element at position `index` if the index is not
    /// out of bounds and the referenced element was not removed before.
    ///
    /// If the element is removed, only the index is marked "deleted". The
    /// actual data is not touched. Thus, the time complexity of this method
    /// is just O(1).
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

    /// Calls `shrink_to_fit()` on the underlying `Vec<T>`.
    ///
    /// Note that this does not moves non-removed elements around and thus does
    /// not invalidates indices. It only calls `shrink_to_fit()` on the
    /// `Vec<T>` that holds the actual data.
    ///
    /// If you want to compact this `StableVec` by removing deleted elements,
    /// use the method `compact()` instead.
    pub fn shrink_to_fit(&mut self) {
        self.data.shrink_to_fit();
    }

    /// Returns the number of non-removed elements in this collection.
    ///
    /// As long as `remove()` is never called, `num_elements()` equals
    /// `next_index()`. Once it is called, `num_elements()` will always be less
    /// than `next_index()`.
    pub fn num_elements(&self) -> usize {
        self.used_count
    }

    /// Returns `true` if this collection doesn't contain any non-removed
    /// items.
    ///
    /// This means that `is_empty()` returns true iff no elements were inserted
    /// *or* all inserted elements were deleted again.
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
