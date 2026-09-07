//@ add-minicore
//@ assembly-output: emit-asm
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -C target-feature=+v -C opt-level=3 -Zcodegen-source-order

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

#[rustc_scalable_vector(2)]
pub struct vbool32_t(bool);
impl Copy for vbool32_t {}

#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vlsegff<SVecTup, Elem>(
    passthru: SVecTup,
    ptr: *const Elem,
    new_vl: *mut usize,
    vl: usize,
) -> SVecTup;

#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vlsegff_mask<SVecTup, Elem, Mask, const POLICY: usize>(
    passthru: SVecTup,
    ptr: *const Elem,
    mask: Mask,
    new_vl: *mut usize,
    vl: usize,
) -> SVecTup;

// CHECK-LABEL: vlseg2e32ff:
// CHECK-NOT: {{vlseg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vlseg2e32ff.v
// CHECK-NOT: {{vlseg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: csrr {{[a-z][a-z0-9]*}}, vl
// CHECK: {{sw|sd}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vlseg2e32ff(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    new_vl: *mut usize,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vlsegff(passthru, ptr, new_vl, vl) }
}

// CHECK-LABEL: vlseg2e32ff_mask:
// CHECK-NOT: {{vlseg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vlseg2e32ff.v {{.*}}, v0.t
// CHECK-NOT: {{vlseg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: csrr [[NEW_VL:[a-z][a-z0-9]*]], vl
// CHECK: {{sw|sd}} [[NEW_VL]], 0({{[a-z][a-z0-9]*}})
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vlseg2e32ff_mask(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    mask: vbool32_t,
    new_vl: *mut usize,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vlsegff_mask::<_, _, _, 3>(passthru, ptr, mask, new_vl, vl) }
}
