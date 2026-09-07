//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -Ctarget-feature=+v -Cno-prepopulate-passes -Zverify-llvm-ir
//@ add-minicore

#![crate_type = "lib"]
#![feature(no_core, riscv_target_feature, rustc_attrs)]
#![no_core]
#![allow(internal_features)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(8)]
pub struct vint8m1_t(i8);
impl Copy for vint8m1_t {}

#[rustc_scalable_vector]
pub struct vint8m1x2_t(vint8m1_t, vint8m1_t);
impl Copy for vint8m1x2_t {}

#[rustc_scalable_vector(1)]
pub struct vint8mf8_t(i8);
impl Copy for vint8mf8_t {}

#[rustc_scalable_vector]
pub struct vint8mf8x8_t(
    vint8mf8_t,
    vint8mf8_t,
    vint8mf8_t,
    vint8mf8_t,
    vint8mf8_t,
    vint8mf8_t,
    vint8mf8_t,
    vint8mf8_t,
);
impl Copy for vint8mf8x8_t {}

#[inline(always)]
unsafe fn undef<T: Copy>() -> T {
    union U<T: Copy> {
        unit: (),
        value: T,
    }

    unsafe { U { unit: () }.value }
}

// CHECK-DAG: store { <vscale x 8 x i8>, <vscale x 8 x i8> } undef
// CHECK-DAG: define{{.*}} @undef_tuple2(
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn undef_tuple2() -> vint8m1x2_t {
    unsafe { undef() }
}

// CHECK-DAG: store { <vscale x 1 x i8>, <vscale x 1 x i8>, <vscale x 1 x i8>, <vscale x 1 x i8>, <vscale x 1 x i8>, <vscale x 1 x i8>, <vscale x 1 x i8>, <vscale x 1 x i8> } undef
// CHECK-DAG: define{{.*}} @undef_tuple8(
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn undef_tuple8() -> vint8mf8x8_t {
    unsafe { undef() }
}
