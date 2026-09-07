//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -C target-feature=+v -C no-prepopulate-passes -Zverify-llvm-ir
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

#[rustc_scalable_vector(4)]
pub struct vuint16m1_t(u16);
impl Copy for vuint16m1_t {}

#[rustc_scalable_vector(64)]
pub struct vbool1_t(bool);
impl Copy for vbool1_t {}

#[rustc_scalable_vector(8)]
pub struct vbool8_t(bool);
impl Copy for vbool8_t {}

#[rustc_scalable_vector(1)]
pub struct vbool64_t(bool);
impl Copy for vbool64_t {}

#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vreinterpret<Src, Dst>(src: Src) -> Dst;

// CHECK-LABEL: define{{.*}} @i8m1_to_b64(
// CHECK: [[FULL:%.*]] = bitcast <vscale x 8 x i8> %src to <vscale x 64 x i1>
// CHECK: [[MASK:%.*]] = call <vscale x 1 x i1> @llvm.vector.extract.nxv1i1.nxv64i1(<vscale x 64 x i1> [[FULL]], i64 0)
// CHECK-NOT: unreachable
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn i8m1_to_b64(src: vint8m1_t) -> vbool64_t {
    unsafe { riscv_vreinterpret(src) }
}

// CHECK-LABEL: define{{.*}} @b64_to_i8m1(
// CHECK: [[FULL:%.*]] = call <vscale x 64 x i1> @llvm.vector.insert.nxv64i1.nxv1i1(<vscale x 64 x i1> poison, <vscale x 1 x i1> %src, i64 0)
// CHECK: [[DATA:%.*]] = bitcast <vscale x 64 x i1> [[FULL]] to <vscale x 8 x i8>
// CHECK-NOT: unreachable
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn b64_to_i8m1(src: vbool64_t) -> vint8m1_t {
    unsafe { riscv_vreinterpret(src) }
}

// CHECK-LABEL: define{{.*}} @u16m1_to_b8(
// CHECK: [[FULL:%.*]] = bitcast <vscale x 4 x i16> %src to <vscale x 64 x i1>
// CHECK: [[MASK:%.*]] = call <vscale x 8 x i1> @llvm.vector.extract.nxv8i1.nxv64i1(<vscale x 64 x i1> [[FULL]], i64 0)
// CHECK-NOT: unreachable
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn u16m1_to_b8(src: vuint16m1_t) -> vbool8_t {
    unsafe { riscv_vreinterpret(src) }
}

// CHECK-LABEL: define{{.*}} @b8_to_u16m1(
// CHECK: [[FULL:%.*]] = call <vscale x 64 x i1> @llvm.vector.insert.nxv64i1.nxv8i1(<vscale x 64 x i1> poison, <vscale x 8 x i1> %src, i64 0)
// CHECK: [[DATA:%.*]] = bitcast <vscale x 64 x i1> [[FULL]] to <vscale x 4 x i16>
// CHECK-NOT: unreachable
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn b8_to_u16m1(src: vbool8_t) -> vuint16m1_t {
    unsafe { riscv_vreinterpret(src) }
}

// Keep the full-width mask on the same canonical insert/extract path.
// CHECK-LABEL: define{{.*}} @i8m1_to_b1(
// CHECK: [[FULL:%.*]] = bitcast <vscale x 8 x i8> %src to <vscale x 64 x i1>
// CHECK: call <vscale x 64 x i1> @llvm.vector.extract.nxv64i1.nxv64i1(<vscale x 64 x i1> [[FULL]], i64 0)
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn i8m1_to_b1(src: vint8m1_t) -> vbool1_t {
    unsafe { riscv_vreinterpret(src) }
}

// CHECK-LABEL: define{{.*}} @b1_to_i8m1(
// CHECK: [[FULL:%.*]] = call <vscale x 64 x i1> @llvm.vector.insert.nxv64i1.nxv64i1(<vscale x 64 x i1> poison, <vscale x 64 x i1> %src, i64 0)
// CHECK: bitcast <vscale x 64 x i1> [[FULL]] to <vscale x 8 x i8>
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn b1_to_i8m1(src: vbool1_t) -> vint8m1_t {
    unsafe { riscv_vreinterpret(src) }
}
