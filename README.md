# stable-vec
[![Build Status](https://img.shields.io/travis/LukasKalbertodt/stable-vec/master.svg)](https://travis-ci.org/LukasKalbertodt/stable-vec)
[![crates.io version](https://img.shields.io/crates/v/stable-vec.svg)](https://crates.io/crates/stable-vec)
[![docs](https://docs.rs/stable-vec/badge.svg)](https://docs.rs/stable-vec)

A `Vec<T>`-like collection which guarantees stable indices and features O(1)
deletion of elements at the cost of wasting some memory. Please refer to the
[**the documentation**](https://docs.rs/stable-vec) for more information.

This crate is still young, but the API won't change a lot.
Everything should already work as intended, but it's not extensively tested yet.
If you're working on mission-critical software, please don't use this library.
Otherwise feel free to do so!

### Alternatives? What about `slab`?

A few weeks after writing the intial version of this crate, I found the crate [`slab`](https://crates.io/crates/slab) which does *pretty much* the same as this crate (and has way more downloads). Despite being very similar, there are a few differences which might be important for you:

- `slab` reuses keys of deleted entries, while `stable-vec` does not; you can only add elements to the back of the stable vector, giving the new element a brand new index/key.
- `slab` does a bit more management internally to quickly know which keys to reuse and where to insert. This might incur a tiny bit of overhead. Most notably: each entry in the underlying `Vec` in `slab` is at least `sizeof::<usize>() + 1` bytes large. If you storing small elements, this might be a significant memory usage overhead.
- `slab` uses only one `Vec` and each element has the information whether or not it is vacant or occupied. `stable-vec` (currently) uses one `Vec<T>` and a `BitVec`. Be aware of the different performance characteristics. (`stable-vec` might switch to `Option<T>` in the future, though).
- The API of `slab` is designed like an associative map, while `stable-vec`'s API is designed more like a `Vec` with additional guarantees. Both crates are probably mostly used as low level building blogs for other things; `stable-vec` might be a tiny bit more low level.

## You want to contribute?

Yes please! This is a rather small crate, but it could still use some developing power.
In particular, you could work on these things:

- Rather easy: implementing new features. See [this issue](https://github.com/LukasKalbertodt/stable-vec/issues/3) for more information.
- Rather easy: write more tests and examples.
- Rather hard: verifying that the use of `unsafe` code is completely fine. It's not a lot of `unsafe` code and I *think* it's fine, but it would be nice to have certainty.

I'm glad to do some mentoring for anyone interested in contributing.

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
