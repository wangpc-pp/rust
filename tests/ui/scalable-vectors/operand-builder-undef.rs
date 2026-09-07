//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: --crate-type=lib -Ctarget-feature=+v -Clink-dead-code -Awarnings -Zverify-llvm-ir
//@ add-minicore
//@ build-pass
//@ dont-check-compiler-stderr

#![feature(no_core, riscv_target_feature, rustc_attrs)]
#![no_core]
#![allow(dead_code, internal_features, non_camel_case_types)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(4)]
pub struct vint16m1_t(i16);
impl Copy for vint16m1_t {}

#[rustc_scalable_vector]
pub struct vint16m1x2_t(vint16m1_t, vint16m1_t);
impl Copy for vint16m1x2_t {}

#[rustc_scalable_vector]
pub struct vint16m1x8_t(
    vint16m1_t,
    vint16m1_t,
    vint16m1_t,
    vint16m1_t,
    vint16m1_t,
    vint16m1_t,
    vint16m1_t,
    vint16m1_t,
);
impl Copy for vint16m1x8_t {}

#[inline(always)]
unsafe fn undef<T: Copy>() -> T {
    union U<T: Copy> {
        unit: (),
        value: T,
    }

    unsafe { U { unit: () }.value }
}

#[target_feature(enable = "v")]
pub unsafe fn undef_vector() -> vint16m1_t {
    unsafe { undef() }
}

#[target_feature(enable = "v")]
pub unsafe fn undef_vector_tuple_x2() -> vint16m1x2_t {
    unsafe { undef() }
}

#[target_feature(enable = "v")]
pub unsafe fn undef_vector_tuple_x8() -> vint16m1x8_t {
    unsafe { undef() }
}
