extern crate bit_vec;

use bit_vec::BitVec;


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
    data: Vec<T>,
    deleted: BitVec,
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
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows `usize`.
    ///
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
        self.deleted.push(true);
        self.used_count += 1;
        self.data.len() - 1
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
}
