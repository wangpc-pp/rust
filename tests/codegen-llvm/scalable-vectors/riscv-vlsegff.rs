//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -C target-feature=+v -C no-prepopulate-passes -Zverify-llvm-ir
//@ compile-flags: -Zcodegen-source-order
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

// CHECK-LABEL: define { <vscale x 2 x i32>, <vscale x 2 x i32> } @vlseg2e32ff(
// CHECK-NOT: sret
// CHECK: [[TUP0:%.*]] = call target("riscv.vector.tuple", <vscale x 8 x i8>, 2) @llvm.riscv.tuple.insert{{.*}}(target("riscv.vector.tuple", <vscale x 8 x i8>, 2) poison, <vscale x 2 x i32> %{{.*}}, i32 0)
// CHECK: [[TUP1:%.*]] = call target("riscv.vector.tuple", <vscale x 8 x i8>, 2) @llvm.riscv.tuple.insert{{.*}}(target("riscv.vector.tuple", <vscale x 8 x i8>, 2) [[TUP0]], <vscale x 2 x i32> %{{.*}}, i32 1)
// CHECK: [[PAIR:%.*]] = call { target("riscv.vector.tuple", <vscale x 8 x i8>, 2), i{{32|64}} } @llvm.riscv.vlseg2ff{{.*}}(target("riscv.vector.tuple", <vscale x 8 x i8>, 2) [[TUP1]], ptr %ptr, i{{32|64}} %vl, i{{32|64}} 5)
// CHECK: [[LOADED:%.*]] = extractvalue { target("riscv.vector.tuple", <vscale x 8 x i8>, 2), i{{32|64}} } [[PAIR]], 0
// CHECK: [[NEW_VL:%.*]] = extractvalue { target("riscv.vector.tuple", <vscale x 8 x i8>, 2), i{{32|64}} } [[PAIR]], 1
// CHECK: store i{{32|64}} [[NEW_VL]], ptr %new_vl
// CHECK: call <vscale x 2 x i32> @llvm.riscv.tuple.extract{{.*}}(target("riscv.vector.tuple", <vscale x 8 x i8>, 2) [[LOADED]], i32 0)
// CHECK: call <vscale x 2 x i32> @llvm.riscv.tuple.extract{{.*}}(target("riscv.vector.tuple", <vscale x 8 x i8>, 2) [[LOADED]], i32 1)
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

// CHECK-LABEL: define { <vscale x 2 x i32>, <vscale x 2 x i32> } @vlseg2e32ff_mask(
// CHECK-NOT: sret
// CHECK: [[MTUP0:%.*]] = call target("riscv.vector.tuple", <vscale x 8 x i8>, 2) @llvm.riscv.tuple.insert{{.*}}(target("riscv.vector.tuple", <vscale x 8 x i8>, 2) poison, <vscale x 2 x i32> %{{.*}}, i32 0)
// CHECK: [[MTUP1:%.*]] = call target("riscv.vector.tuple", <vscale x 8 x i8>, 2) @llvm.riscv.tuple.insert{{.*}}(target("riscv.vector.tuple", <vscale x 8 x i8>, 2) [[MTUP0]], <vscale x 2 x i32> %{{.*}}, i32 1)
// CHECK: [[MPAIR:%.*]] = call { target("riscv.vector.tuple", <vscale x 8 x i8>, 2), i{{32|64}} } @llvm.riscv.vlseg2ff.mask{{.*}}(target("riscv.vector.tuple", <vscale x 8 x i8>, 2) [[MTUP1]], ptr %ptr, <vscale x 2 x i1> %mask, i{{32|64}} %vl, i{{32|64}} 3, i{{32|64}} 5)
// CHECK: [[MLOADED:%.*]] = extractvalue { target("riscv.vector.tuple", <vscale x 8 x i8>, 2), i{{32|64}} } [[MPAIR]], 0
// CHECK: [[MNEW_VL:%.*]] = extractvalue { target("riscv.vector.tuple", <vscale x 8 x i8>, 2), i{{32|64}} } [[MPAIR]], 1
// CHECK: store i{{32|64}} [[MNEW_VL]], ptr %new_vl
// CHECK: call <vscale x 2 x i32> @llvm.riscv.tuple.extract{{.*}}(target("riscv.vector.tuple", <vscale x 8 x i8>, 2) [[MLOADED]], i32 0)
// CHECK: call <vscale x 2 x i32> @llvm.riscv.tuple.extract{{.*}}(target("riscv.vector.tuple", <vscale x 8 x i8>, 2) [[MLOADED]], i32 1)
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
