// ignore-tidy-linelength
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -C target-feature=+v,+zvfbfmin,+zvfbfa,+zvfofp8min -C no-prepopulate-passes -Zverify-llvm-ir
//@ add-minicore

#![crate_type = "lib"]
#![feature(link_llvm_intrinsics, no_core, rustc_attrs)]
#![no_core]
#![allow(internal_features)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(1)]
pub struct vbfloat16mf4_t(u16);
impl Copy for vbfloat16mf4_t {}

#[rustc_scalable_vector(1)]
pub struct vfloat32mf2_t(f32);
impl Copy for vfloat32mf2_t {}

#[rustc_scalable_vector(4)]
pub struct vbfloat16m1_t(u16);
impl Copy for vbfloat16m1_t {}

#[rustc_scalable_vector(4)]
pub struct vfloat32m2_t(f32);
impl Copy for vfloat32m2_t {}

#[rustc_scalable_vector(4)]
pub struct vbool16_t(bool);
impl Copy for vbool16_t {}

#[rustc_scalable_vector(4)]
pub struct vuint16m1_t(u16);
impl Copy for vuint16m1_t {}

#[rustc_scalable_vector(8)]
pub struct vbfloat16m2_t(u16);
impl Copy for vbfloat16m2_t {}

#[rustc_scalable_vector(8)]
pub struct vfloat8e4m3m1_t(u8);
impl Copy for vfloat8e4m3m1_t {}

// CHECK-LABEL: @widen_bf16_to_f32(
// CHECK: [[BF16:%.*]] = bitcast <vscale x 1 x i16> %src to <vscale x 1 x bfloat>
// RV32: call <vscale x 1 x float> @llvm.riscv.vfwcvtbf16.f.f.v.nxv1f32.nxv1bf16.i32(<vscale x 1 x float> poison, <vscale x 1 x bfloat> [[BF16]], i32 %vl)
// RV64: call <vscale x 1 x float> @llvm.riscv.vfwcvtbf16.f.f.v.nxv1f32.nxv1bf16.i64(<vscale x 1 x float> poison, <vscale x 1 x bfloat> [[BF16]], i64 %vl)
#[unsafe(no_mangle)]
pub unsafe fn widen_bf16_to_f32(src: vbfloat16mf4_t, vl: usize) -> vfloat32mf2_t {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(
            target_arch = "riscv32",
            link_name = "llvm.riscv.vfwcvtbf16.f.f.v.nxv1f32.nxv1bf16.i32"
        )]
        #[cfg_attr(
            target_arch = "riscv64",
            link_name = "llvm.riscv.vfwcvtbf16.f.f.v.nxv1f32.nxv1bf16.i64"
        )]
        fn intrinsic(passthru: vfloat32mf2_t, src: vbfloat16mf4_t, vl: usize) -> vfloat32mf2_t;
    }
    unsafe { intrinsic(undef(), src, vl) }
}

// CHECK-LABEL: @narrow_f32_to_bf16(
// RV32: [[BF16:%.*]] = call <vscale x 1 x bfloat> @llvm.riscv.vfncvtbf16.f.f.w.nxv1bf16.nxv1f32.i32(<vscale x 1 x bfloat> poison, <vscale x 1 x float> %src, i32 7, i32 %vl)
// RV64: [[BF16:%.*]] = call <vscale x 1 x bfloat> @llvm.riscv.vfncvtbf16.f.f.w.nxv1bf16.nxv1f32.i64(<vscale x 1 x bfloat> poison, <vscale x 1 x float> %src, i64 7, i64 %vl)
// CHECK: bitcast <vscale x 1 x bfloat> %{{.*}} to <vscale x 1 x i16>
#[unsafe(no_mangle)]
pub unsafe fn narrow_f32_to_bf16(src: vfloat32mf2_t, vl: usize) -> vbfloat16mf4_t {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(
            target_arch = "riscv32",
            link_name = "llvm.riscv.vfncvtbf16.f.f.w.nxv1bf16.nxv1f32.i32"
        )]
        #[cfg_attr(
            target_arch = "riscv64",
            link_name = "llvm.riscv.vfncvtbf16.f.f.w.nxv1bf16.nxv1f32.i64"
        )]
        fn intrinsic(
            passthru: vbfloat16mf4_t,
            src: vfloat32mf2_t,
            frm: usize,
            vl: usize,
        ) -> vbfloat16mf4_t;
    }
    unsafe { intrinsic(undef(), src, 7, vl) }
}

// CHECK-LABEL: @masked_widen_bf16_to_f32(
// CHECK: [[BF16:%.*]] = bitcast <vscale x 4 x i16> %src to <vscale x 4 x bfloat>
// RV32: call <vscale x 4 x float> @llvm.riscv.vfwcvtbf16.f.f.v.mask.nxv4f32.nxv4bf16.i32(<vscale x 4 x float> poison, <vscale x 4 x bfloat> [[BF16]], <vscale x 4 x i1> %mask, i32 %vl, i32 3)
// RV64: call <vscale x 4 x float> @llvm.riscv.vfwcvtbf16.f.f.v.mask.nxv4f32.nxv4bf16.i64(<vscale x 4 x float> poison, <vscale x 4 x bfloat> [[BF16]], <vscale x 4 x i1> %mask, i64 %vl, i64 3)
#[unsafe(no_mangle)]
pub unsafe fn masked_widen_bf16_to_f32(
    mask: vbool16_t,
    src: vbfloat16m1_t,
    vl: usize,
) -> vfloat32m2_t {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(
            target_arch = "riscv32",
            link_name = "llvm.riscv.vfwcvtbf16.f.f.v.mask.nxv4f32.nxv4bf16.i32"
        )]
        #[cfg_attr(
            target_arch = "riscv64",
            link_name = "llvm.riscv.vfwcvtbf16.f.f.v.mask.nxv4f32.nxv4bf16.i64"
        )]
        fn intrinsic(
            passthru: vfloat32m2_t,
            src: vbfloat16m1_t,
            mask: vbool16_t,
            vl: usize,
            policy: usize,
        ) -> vfloat32m2_t;
    }
    unsafe { intrinsic(undef(), src, mask, vl, 3) }
}

// CHECK-LABEL: @vfwmaccbf16_vv(
// CHECK: [[LHS:%.*]] = bitcast <vscale x 4 x i16> %lhs to <vscale x 4 x bfloat>
// CHECK: [[RHS:%.*]] = bitcast <vscale x 4 x i16> %rhs to <vscale x 4 x bfloat>
// RV32: call <vscale x 4 x float> @llvm.riscv.vfwmaccbf16.nxv4f32.nxv4bf16.nxv4bf16.i32(<vscale x 4 x float> %acc, <vscale x 4 x bfloat> [[LHS]], <vscale x 4 x bfloat> [[RHS]], i32 7, i32 %vl, i32 3)
// RV64: call <vscale x 4 x float> @llvm.riscv.vfwmaccbf16.nxv4f32.nxv4bf16.nxv4bf16.i64(<vscale x 4 x float> %acc, <vscale x 4 x bfloat> [[LHS]], <vscale x 4 x bfloat> [[RHS]], i64 7, i64 %vl, i64 3)
#[unsafe(no_mangle)]
pub unsafe fn vfwmaccbf16_vv(
    acc: vfloat32m2_t,
    lhs: vbfloat16m1_t,
    rhs: vbfloat16m1_t,
    vl: usize,
) -> vfloat32m2_t {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(
            target_arch = "riscv32",
            link_name = "llvm.riscv.vfwmaccbf16.nxv4f32.nxv4bf16.nxv4bf16.i32"
        )]
        #[cfg_attr(
            target_arch = "riscv64",
            link_name = "llvm.riscv.vfwmaccbf16.nxv4f32.nxv4bf16.nxv4bf16.i64"
        )]
        fn intrinsic(
            acc: vfloat32m2_t,
            lhs: vbfloat16m1_t,
            rhs: vbfloat16m1_t,
            frm: usize,
            vl: usize,
            policy: usize,
        ) -> vfloat32m2_t;
    }
    unsafe { intrinsic(acc, lhs, rhs, 7, vl, 3) }
}

// CHECK-LABEL: @vfwmaccbf16_vf(
// CHECK: [[SCALAR:%.*]] = bitcast i16 %lhs to bfloat
// CHECK: [[VECTOR:%.*]] = bitcast <vscale x 4 x i16> %rhs to <vscale x 4 x bfloat>
// RV32: call <vscale x 4 x float> @llvm.riscv.vfwmaccbf16.nxv4f32.bf16.nxv4bf16.i32(<vscale x 4 x float> %acc, bfloat [[SCALAR]], <vscale x 4 x bfloat> [[VECTOR]], i32 7, i32 %vl, i32 3)
// RV64: call <vscale x 4 x float> @llvm.riscv.vfwmaccbf16.nxv4f32.bf16.nxv4bf16.i64(<vscale x 4 x float> %acc, bfloat [[SCALAR]], <vscale x 4 x bfloat> [[VECTOR]], i64 7, i64 %vl, i64 3)
#[unsafe(no_mangle)]
pub unsafe fn vfwmaccbf16_vf(
    acc: vfloat32m2_t,
    lhs: u16,
    rhs: vbfloat16m1_t,
    vl: usize,
) -> vfloat32m2_t {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(
            target_arch = "riscv32",
            link_name = "llvm.riscv.vfwmaccbf16.nxv4f32.bf16.nxv4bf16.i32"
        )]
        #[cfg_attr(
            target_arch = "riscv64",
            link_name = "llvm.riscv.vfwmaccbf16.nxv4f32.bf16.nxv4bf16.i64"
        )]
        fn intrinsic(
            acc: vfloat32m2_t,
            lhs: u16,
            rhs: vbfloat16m1_t,
            frm: usize,
            vl: usize,
            policy: usize,
        ) -> vfloat32m2_t;
    }
    unsafe { intrinsic(acc, lhs, rhs, 7, vl, 3) }
}

// CHECK-LABEL: @classify_bf16(
// CHECK: [[BF16:%.*]] = bitcast <vscale x 4 x i16> %src to <vscale x 4 x bfloat>
// RV32: call <vscale x 4 x i16> @llvm.riscv.vfclass.nxv4bf16.i32(<vscale x 4 x i16> poison, <vscale x 4 x bfloat> [[BF16]], i32 %vl)
// RV64: call <vscale x 4 x i16> @llvm.riscv.vfclass.nxv4bf16.i64(<vscale x 4 x i16> poison, <vscale x 4 x bfloat> [[BF16]], i64 %vl)
#[unsafe(no_mangle)]
pub unsafe fn classify_bf16(src: vbfloat16m1_t, vl: usize) -> vuint16m1_t {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(target_arch = "riscv32", link_name = "llvm.riscv.vfclass.nxv4bf16.i32")]
        #[cfg_attr(target_arch = "riscv64", link_name = "llvm.riscv.vfclass.nxv4bf16.i64")]
        fn intrinsic(passthru: vuint16m1_t, src: vbfloat16m1_t, vl: usize) -> vuint16m1_t;
    }
    unsafe { intrinsic(undef(), src, vl) }
}

// CHECK-LABEL: @move_bf16_from_vector(
// CHECK: [[BF16:%.*]] = bitcast <vscale x 4 x i16> %src to <vscale x 4 x bfloat>
// CHECK: [[SCALAR:%.*]] = call bfloat @llvm.riscv.vfmv.f.s.nxv4bf16(<vscale x 4 x bfloat> [[BF16]])
// CHECK: bitcast bfloat [[SCALAR]] to i16
#[unsafe(no_mangle)]
pub unsafe fn move_bf16_from_vector(src: vbfloat16m1_t) -> u16 {
    unsafe extern "llvm-intrinsic" {
        #[link_name = "llvm.riscv.vfmv.f.s.nxv4bf16"]
        fn intrinsic(src: vbfloat16m1_t) -> u16;
    }
    unsafe { intrinsic(src) }
}

// CHECK-LABEL: @widen_ofp8_to_bf16(
// CHECK: [[PASSTHRU:%.*]] = bitcast <vscale x 8 x i16> %{{.*}} to <vscale x 8 x bfloat>
// CHECK: [[OFP8_RESULT:%.*]] = call <vscale x 8 x bfloat> @llvm.riscv.vfwcvt.f.f.v.nxv8bf16.nxv8i8.{{i32|i64}}(<vscale x 8 x bfloat> [[PASSTHRU]], <vscale x 8 x i8> %src, {{i32|i64}} %vl)
// CHECK: bitcast <vscale x 8 x bfloat> [[OFP8_RESULT]] to <vscale x 8 x i16>
#[unsafe(no_mangle)]
pub unsafe fn widen_ofp8_to_bf16(src: vfloat8e4m3m1_t, vl: usize) -> vbfloat16m2_t {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(
            target_arch = "riscv32",
            link_name = "llvm.riscv.vfwcvt.f.f.v.nxv8bf16.nxv8i8.i32"
        )]
        #[cfg_attr(
            target_arch = "riscv64",
            link_name = "llvm.riscv.vfwcvt.f.f.v.nxv8bf16.nxv8i8.i64"
        )]
        fn intrinsic(passthru: vbfloat16m2_t, src: vfloat8e4m3m1_t, vl: usize) -> vbfloat16m2_t;
    }
    unsafe { intrinsic(undef(), src, vl) }
}

// CHECK-LABEL: @narrow_bf16_to_ofp8(
// CHECK: [[BF16:%.*]] = bitcast <vscale x 8 x i16> %src to <vscale x 8 x bfloat>
// RV32: call <vscale x 8 x i8> @llvm.riscv.vfncvt.f.f.w.nxv8i8.nxv8bf16.i32(<vscale x 8 x i8> poison, <vscale x 8 x bfloat> [[BF16]], i32 7, i32 %vl)
// RV64: call <vscale x 8 x i8> @llvm.riscv.vfncvt.f.f.w.nxv8i8.nxv8bf16.i64(<vscale x 8 x i8> poison, <vscale x 8 x bfloat> [[BF16]], i64 7, i64 %vl)
#[unsafe(no_mangle)]
pub unsafe fn narrow_bf16_to_ofp8(src: vbfloat16m2_t, vl: usize) -> vfloat8e4m3m1_t {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(
            target_arch = "riscv32",
            link_name = "llvm.riscv.vfncvt.f.f.w.nxv8i8.nxv8bf16.i32"
        )]
        #[cfg_attr(
            target_arch = "riscv64",
            link_name = "llvm.riscv.vfncvt.f.f.w.nxv8i8.nxv8bf16.i64"
        )]
        fn intrinsic(
            passthru: vfloat8e4m3m1_t,
            src: vbfloat16m2_t,
            frm: usize,
            vl: usize,
        ) -> vfloat8e4m3m1_t;
    }
    unsafe { intrinsic(undef(), src, 7, vl) }
}

unsafe fn undef<T: Copy>() -> T {
    union U<T: Copy> {
        unit: (),
        value: T,
    }
    unsafe { U { unit: () }.value }
}
