//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -C target-feature=+v -C no-prepopulate-passes
//@ add-minicore

#![crate_type = "lib"]
#![feature(intrinsics, no_core, riscv_target_feature, rustc_attrs)]
#![no_core]
#![allow(internal_features)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(8)]
pub struct vint8m1_t(i8);
impl Copy for vint8m1_t {}

#[rustc_scalable_vector(8)]
pub struct vbool8_t(bool);
impl Copy for vbool8_t {}

#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vleff<SVec, Elem>(
    passthru: SVec,
    ptr: *const Elem,
    new_vl: *mut usize,
    vl: usize,
) -> SVec;

#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vleff_mask<SVec, Elem, Mask, const POLICY: usize>(
    passthru: SVec,
    ptr: *const Elem,
    mask: Mask,
    new_vl: *mut usize,
    vl: usize,
) -> SVec;

// CHECK-LABEL: define <vscale x 8 x i8> @vle8ff
// CHECK: [[PAIR:%.*]] = call { <vscale x 8 x i8>, i{{32|64}} } @llvm.riscv.vleff.nxv8i8.i{{32|64}}.p0(
// CHECK-SAME: <vscale x 8 x i8> %passthru, ptr %ptr, i{{32|64}} %vl)
// CHECK: [[VECTOR:%.*]] = extractvalue { <vscale x 8 x i8>, i{{32|64}} } [[PAIR]], 0
// CHECK: [[NEW_VL:%.*]] = extractvalue { <vscale x 8 x i8>, i{{32|64}} } [[PAIR]], 1
// CHECK: store i{{32|64}} [[NEW_VL]], ptr %new_vl
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vle8ff(
    passthru: vint8m1_t,
    ptr: *const i8,
    new_vl: *mut usize,
    vl: usize,
) -> vint8m1_t {
    unsafe { riscv_vleff(passthru, ptr, new_vl, vl) }
}

// CHECK-LABEL: define <vscale x 8 x i8> @vle8ff_mask
// CHECK: [[PAIR:%.*]] = call { <vscale x 8 x i8>, i{{32|64}} } @llvm.riscv.vleff.mask.nxv8i8.i{{32|64}}.p0(
// CHECK-SAME: <vscale x 8 x i8> %passthru, ptr %ptr, <vscale x 8 x i1> %mask, i{{32|64}} %vl, i{{32|64}} 3)
// CHECK: [[VECTOR:%.*]] = extractvalue { <vscale x 8 x i8>, i{{32|64}} } [[PAIR]], 0
// CHECK: [[NEW_VL:%.*]] = extractvalue { <vscale x 8 x i8>, i{{32|64}} } [[PAIR]], 1
// CHECK: store i{{32|64}} [[NEW_VL]], ptr %new_vl
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vle8ff_mask(
    passthru: vint8m1_t,
    ptr: *const i8,
    mask: vbool8_t,
    new_vl: *mut usize,
    vl: usize,
) -> vint8m1_t {
    unsafe { riscv_vleff_mask::<_, _, _, 3>(passthru, ptr, mask, new_vl, vl) }
}
