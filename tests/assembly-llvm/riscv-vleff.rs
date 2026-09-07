//@ add-minicore
//@ assembly-output: emit-asm
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -C target-feature=+v -C opt-level=3

#![crate_type = "lib"]
#![feature(intrinsics, no_core, riscv_target_feature, rustc_attrs)]
#![no_core]
#![allow(internal_features)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(2)]
pub struct vint32m1_t(i32);
impl Copy for vint32m1_t {}

#[rustc_scalable_vector(2)]
pub struct vbool32_t(bool);
impl Copy for vbool32_t {}

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

// CHECK-LABEL: vle32ff:
// CHECK: vle32ff.v
// CHECK: csrr [[VLE_NEW_VL:[a-z][a-z0-9]*]], vl
// CHECK: {{sw|sd}} [[VLE_NEW_VL]], 0({{[a-z][a-z0-9]*}})
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vle32ff(
    passthru: vint32m1_t,
    ptr: *const i32,
    new_vl: *mut usize,
    vl: usize,
) -> vint32m1_t {
    unsafe { riscv_vleff(passthru, ptr, new_vl, vl) }
}

// CHECK-LABEL: vle32ff_mask:
// CHECK: vle32ff.v {{.*}}, v0.t
// CHECK: csrr [[VLE_MASK_NEW_VL:[a-z][a-z0-9]*]], vl
// CHECK: {{sw|sd}} [[VLE_MASK_NEW_VL]], 0({{[a-z][a-z0-9]*}})
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vle32ff_mask(
    passthru: vint32m1_t,
    ptr: *const i32,
    mask: vbool32_t,
    new_vl: *mut usize,
    vl: usize,
) -> vint32m1_t {
    unsafe { riscv_vleff_mask::<_, _, _, 3>(passthru, ptr, mask, new_vl, vl) }
}
