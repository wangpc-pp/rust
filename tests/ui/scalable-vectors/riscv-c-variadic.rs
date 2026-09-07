//@ add-minicore
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv

#![crate_type = "lib"]
#![feature(no_core, rustc_attrs)]
#![no_core]
#![allow(internal_features, non_camel_case_types)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(2)]
struct vint32m1_t(i32);
impl Copy for vint32m1_t {}

#[rustc_scalable_vector]
struct vint32m1x2_t(vint32m1_t, vint32m1_t);
impl Copy for vint32m1x2_t {}

unsafe extern "C" {
    fn variadic(_: i32, ...);
}

unsafe fn pass_scalable_vector(vector: vint32m1_t) {
    unsafe { variadic(0, vector) };
    //~^ ERROR can't pass scalable vector type `vint32m1_t` to variadic function
}

unsafe fn pass_scalable_vector_tuple(tuple: vint32m1x2_t) {
    unsafe { variadic(0, tuple) };
    //~^ ERROR can't pass scalable vector type `vint32m1x2_t` to variadic function
}
