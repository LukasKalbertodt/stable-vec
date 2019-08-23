# stable-vec
[![Build Status](https://img.shields.io/travis/LukasKalbertodt/stable-vec/master.svg)](https://travis-ci.org/LukasKalbertodt/stable-vec)
[![crates.io version](https://img.shields.io/crates/v/stable-vec.svg)](https://crates.io/crates/stable-vec)
[![docs](https://docs.rs/stable-vec/badge.svg)](https://docs.rs/stable-vec)

A `Vec<T>`-like collection which guarantees stable indices and features O(1) element removal at the cost of wasting some memory.
It is semantically very similar to `Vec<Option<T>>`, but with a more optimized memory layout and a more convenient API.
This data structure is very useful as a foundation to implement other data structures like graphs and polygon meshes.
In those situations, `stable-vec` functions a bit like an arena memory allocator.
This crate has no dependencies and works in `#![no_std]` context (it needs the `alloc` crate, though).

This crate implements different strategies to store the information.
As these strategies have slightly different performance characteristics, the user can choose which to use.
The two main strategies are:
- something similar to `Vec<T>` with a `BitVec` (used by default), and
- something similar to `Vec<Option<T>>`.

Please refer to [**the documentation**](https://docs.rs/stable-vec) for more information. Example:

```rust
let mut sv = StableVec::new();
let star_idx = sv.push('★');
let heart_idx = sv.push('♥');
let lamda_idx = sv.push('λ');

// Deleting an element does not invalidate any other indices.
sv.remove(star_idx);
assert_eq!(sv[heart_idx], '♥');
assert_eq!(sv[lamda_idx], 'λ');

// You can insert into empty slots (again, without invalidating any indices)
sv.insert(star_idx, '☺');

// We can also reserve memory (create new empty slots) and insert into
// these new slots. All slots up to `sv.capacity()` can be accessed.
sv.reserve_for(15);
assert_eq!(sv.get(15), None);
sv.insert(15, '☮');

// The final state of the stable vec
assert_eq!(sv.get(0), Some(&'☺'));
assert_eq!(sv.get(1), Some(&'♥'));
assert_eq!(sv.get(2), Some(&'λ'));
assert_eq!(sv.get(3), None);
assert_eq!(sv.get(14), None);
assert_eq!(sv.get(15), Some(&'☮'));
```


### Alternatives? What about `slab`?

The crate [`slab`](https://crates.io/crates/slab) works very similar to `stable-vec`, but has way more downloads.
Despite being very similar, there are a few differences which might be important for you:

- `slab` reuses keys of deleted entries, while `stable-vec` does not automatically.
- `slab` does a bit more management internally to quickly know which keys to reuse and where to insert.
  This might incur a tiny bit of overhead.
  Most notably: each entry in the underlying `Vec` in `slab` is at least `size_of::<usize>() + 1` bytes large.
  If you're storing small elements, this might be a significant memory usage overhead.
- `slab` has a fixed memory layout while `stable-vec` lets you choose between different layouts.
  These have different performance characteristics and you might want to choose the right one for your situation.
- The API of `stable-vec` is a bit more low level.

---

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
