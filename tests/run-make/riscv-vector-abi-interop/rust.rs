#![crate_type = "lib"]
#![feature(no_core, rustc_attrs)]
#![no_core]
#![allow(dead_code, internal_features)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(2)]
pub struct Vint32m1(i32);
impl Copy for Vint32m1 {}

#[rustc_scalable_vector]
pub struct Vint32m1x2(Vint32m1, Vint32m1);
impl Copy for Vint32m1x2 {}

unsafe extern "C" {
    fn c_vector_add(x: Vint32m1, delta: i32, vl: usize) -> Vint32m1;
    fn c_tuple_add(x: Vint32m1x2, vl: usize) -> Vint32m1x2;
}

// Rust definitions called from C. These identity/select operations leave the values observable in
// C without requiring Rust-side RVV intrinsics in this no-core test.
#[unsafe(no_mangle)]
pub extern "C" fn rust_vector_identity(x: Vint32m1) -> Vint32m1 {
    x
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_tuple_identity(x: Vint32m1x2) -> Vint32m1x2 {
    x
}

// C definitions called from Rust. C calls these wrappers, so a successful result exercises both
// sides of each cross-language boundary in one executable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_calls_c_vector(x: Vint32m1, delta: i32, vl: usize) -> Vint32m1 {
    unsafe { c_vector_add(x, delta, vl) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rust_calls_c_tuple(x: Vint32m1x2, vl: usize) -> Vint32m1x2 {
    unsafe { c_tuple_add(x, vl) }
}
