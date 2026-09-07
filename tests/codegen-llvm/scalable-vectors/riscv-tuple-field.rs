//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -Ctarget-feature=+v -Cno-prepopulate-passes -Zverify-llvm-ir
//@ add-minicore

#![crate_type = "lib"]
#![feature(intrinsics, no_core, riscv_target_feature, rustc_attrs)]
#![no_core]
#![allow(internal_features)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(2)]
pub struct vint32m1_t(i32);
impl Copy for vint32m1_t {}

#[rustc_scalable_vector]
pub struct vint32m1x2_t(vint32m1_t, vint32m1_t);
impl Copy for vint32m1x2_t {}

#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn sve_tuple_get<SVecTup, SVec, const IDX: i32>(tuple: SVecTup) -> SVec;

// CHECK-LABEL: define <vscale x 2 x i32> @extract_second
// CHECK-SAME: ({ <vscale x 2 x i32>, <vscale x 2 x i32> } %tuple)
// CHECK: extractvalue { <vscale x 2 x i32>, <vscale x 2 x i32> } %tuple, 1
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub fn extract_second(tuple: vint32m1x2_t) -> vint32m1_t {
    unsafe { sve_tuple_get::<_, _, 1>(tuple) }
}
