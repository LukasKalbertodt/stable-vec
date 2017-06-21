# stable-vec
[![Build Status](https://img.shields.io/travis/LukasKalbertodt/stable-vec/master.svg)](https://travis-ci.org/LukasKalbertodt/stable-vec)
[![crates.io version](https://img.shields.io/crates/v/stable-vec.svg)](https://crates.io/crates/stable-vec)

A `Vec<T>`-like collection which guarantees stable indices and features O(1) deletion of elements at the cost of wasting some memory.
Please refer to the [**the documentation**](https://docs.rs/stable-vec) for more information.

This crate is still young, but the API won't change a lot.
Everything should already work as intended, but it's not extensively tested yet.
If you're working on mission-critical software, please don't use this library.
Otherwise feel free to do so!

## You want to contribute?

Yes please! This is a rather small crate, but it could still use some developing power.
In particular, I'm mostly interested in these two things:

- Rather easy: implementing new features. See [this issue](https://github.com/LukasKalbertodt/stable-vec/issues/3) for more information.
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
