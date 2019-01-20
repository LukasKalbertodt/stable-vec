#[macro_use]
extern crate criterion;
extern crate stable_vec;

use criterion::Criterion;
use stable_vec::StableVec;
use std::{
    iter::FromIterator,
};


// ===========================================================================
// ===== Functions to generate instances with a given size
// ===========================================================================
fn full_sv(size: usize) -> StableVec<u32> {
    (0..size as u32).collect()
}

fn sv_with_hole_in_middle(size: usize) -> StableVec<u32> {
    let mut sv = full_sv(size);
    for i in size / 3..2 * size / 3 {
        sv.remove(i);
    }
    sv
}

fn sv_with_hole_every_fifth(size: usize) -> StableVec<u32> {
    let mut sv = full_sv(size);
    for i in (0..size).step_by(5) {
        sv.remove(i);
    }
    sv
}

fn sv_with_element_every_fifth(size: usize) -> StableVec<u32> {
    let mut sv = full_sv(size);
    for i in 0..size {
        if i % 5 != 0 {
            sv.remove(i);
        }
    }
    sv
}

fn sv_with_prime_holes(size: usize) -> StableVec<u32> {
    fn is_prime(n: u32) -> bool {
        let upper = (n as f64).sqrt() as u32;
        (2..upper).all(|d| n % d != 0)
    }

    let mut sv = full_sv(size);
    for i in 0..size {
        if is_prime(i as u32) {
            sv.remove(i);
        }
    }

    sv
}

fn two_element_sv(size: usize) -> StableVec<u32> {
    let mut sv = full_sv(size);
    for i in 0..size {
        if i != size / 4 && i != 3 * size / 4  {
            sv.remove(i);
        }
    }

    sv
}

fn fully_deleted_sv(size: usize) -> StableVec<u32> {
    let mut sv = full_sv(size);
    for i in 0..size {
        sv.remove(i);
    }

    sv
}


// ===========================================================================
// ===== The actual benchmarks
// ===========================================================================

fn clear(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "clear",
        |b, size| {
            b.iter_with_setup(
                || full_sv(*size),
                |mut sv| {
                    sv.clear();
                    sv
                },
            );
        },
        vec![0, 1, 10, 1000, 100_000],
    );
}

fn from_vec(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "from_vec",
        |b, size| {
            b.iter_with_setup(
                || Vec::from_iter(0..*size),
                |v| StableVec::from_vec(v),
            );
        },
        vec![0, 1, 10, 1000, 100_000],
    );
}

fn push(c: &mut Criterion) {
    c.bench_function("push", |b| {
        b.iter_with_setup(
            || StableVec::with_capacity(1),
            |mut sv| {
                sv.push('x');
                sv
            },
        );
    });
}

fn delete_some_elements(c: &mut Criterion) {
    /// Some arbitrary delete condition
    fn should_delete(i: usize) -> bool {
        i % 13 == 0
            || (i / 3) % 7 == 0
            || (i / 10) % 3 == 0
    }

    c.bench_function_over_inputs(
        "delete_some_elements_from_full",
        |b, &len| {
            b.iter_with_setup(
                || full_sv(len),
                |mut sv| {
                    for i in 0..sv.next_index() {
                        if should_delete(i) {
                            sv.remove(i);
                        }
                    }
                    sv
                },
            );
        },
        vec![10, 1000, 100_000],
    );

    c.bench_function_over_inputs(
        "delete_some_elements_from_prime_holes",
        |b, &len| {
            b.iter_with_setup(
                || sv_with_prime_holes(len),
                |mut sv| {
                    for i in 0..sv.next_index() {
                        if should_delete(i) {
                            sv.remove(i);
                        }
                    }
                    sv
                },
            );
        },
        vec![10, 1000, 100_000],
    );
}

fn get(c: &mut Criterion) {
    const SIZE: usize = 100_000;
    let sv = full_sv(SIZE);
    c.bench_function("get_full", move |b| {
        b.iter(|| sv.get(SIZE / 3));
    });

    let sv = sv_with_hole_in_middle(SIZE);
    c.bench_function("get_hit_hole_in_middle", move |b| {
        b.iter(|| sv.get(3 * SIZE / 4));
    });

    let sv = sv_with_hole_in_middle(SIZE);
    c.bench_function("get_miss_hole_in_middle", move |b| {
        b.iter(|| sv.get(SIZE / 2));
    });
}

fn count(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "count_full",
        move |b, &len| {
            let sv = full_sv(len);
            b.iter(|| sv.iter().count());
        },
        vec![0, 1, 10, 1000, 100_000],
    );

    c.bench_function_over_inputs(
        "count_with_one_hole",
        move |b, &len| {
            let sv = sv_with_hole_in_middle(len);
            b.iter(|| sv.iter().count());
        },
        vec![10, 1000, 100_000],
    );

    c.bench_function_over_inputs(
        "count_with_hole_every_fifth",
        move |b, &len| {
            let sv = sv_with_hole_every_fifth(len);
            b.iter(|| sv.iter().count());
        },
        vec![10, 1000, 100_000],
    );

    c.bench_function_over_inputs(
        "count_with_element_every_fifth",
        move |b, &len| {
            let sv = sv_with_element_every_fifth(len);
            b.iter(|| sv.iter().count());
        },
        vec![10, 1000, 100_000],
    );

    c.bench_function_over_inputs(
        "count_with_prime_holes",
        move |b, &len| {
            let sv = sv_with_prime_holes(len);
            b.iter(|| sv.iter().count());
        },
        vec![10, 1000, 100_000],
    );

    c.bench_function_over_inputs(
        "count_two_elements",
        move |b, &len| {
            let sv = two_element_sv(len);
            b.iter(|| sv.iter().count());
        },
        vec![10, 1000, 100_000],
    );

    c.bench_function_over_inputs(
        "count_fully_deleted",
        move |b, &len| {
            let sv = fully_deleted_sv(len);
            b.iter(|| sv.iter().count());
        },
        vec![10, 1000, 100_000],
    );
}

fn sum(c: &mut Criterion) {
    c.bench_function_over_inputs(
        "sum_full",
        move |b, &len| {
            let sv = full_sv(len);
            b.iter(|| sv.iter().map(|&e| e as u64).sum::<u64>());
        },
        vec![0, 1, 10, 1000, 100_000],
    );

    c.bench_function_over_inputs(
        "sum_with_one_hole",
        move |b, &len| {
            let sv = sv_with_hole_in_middle(len);
            b.iter(|| sv.iter().map(|&e| e as u64).sum::<u64>());
        },
        vec![10, 1000, 100_000],
    );

    c.bench_function_over_inputs(
        "sum_with_hole_every_fifth",
        move |b, &len| {
            let sv = sv_with_hole_every_fifth(len);
            b.iter(|| sv.iter().map(|&e| e as u64).sum::<u64>());
        },
        vec![10, 1000, 100_000],
    );

    c.bench_function_over_inputs(
        "sum_with_element_every_fifth",
        move |b, &len| {
            let sv = sv_with_element_every_fifth(len);
            b.iter(|| sv.iter().map(|&e| e as u64).sum::<u64>());
        },
        vec![10, 1000, 100_000],
    );

    c.bench_function_over_inputs(
        "sum_with_prime_holes",
        move |b, &len| {
            let sv = sv_with_prime_holes(len);
            b.iter(|| sv.iter().map(|&e| e as u64).sum::<u64>());
        },
        vec![10, 1000, 100_000],
    );

    c.bench_function_over_inputs(
        "sum_two_elements",
        move |b, &len| {
            let sv = two_element_sv(len);
            b.iter(|| sv.iter().map(|&e| e as u64).sum::<u64>());
        },
        vec![10, 1000, 100_000],
    );

    c.bench_function_over_inputs(
        "sum_fully_deleted",
        move |b, &len| {
            let sv = fully_deleted_sv(len);
            b.iter(|| sv.iter().map(|&e| e as u64).sum::<u64>());
        },
        vec![10, 1000, 100_000],
    );
}


criterion_group!(
    benches,
    clear,
    from_vec,
    push,
    delete_some_elements,
    get,
    count,
    sum,
);
criterion_main!(benches);
