//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -C target-feature=+v -C no-prepopulate-passes -Zverify-llvm-ir
//@ compile-flags: -Zcodegen-source-order
//@ add-minicore

#![crate_type = "lib"]
#![feature(link_llvm_intrinsics, no_core, riscv_target_feature, rustc_attrs)]
#![no_core]
#![allow(internal_features)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(8)]
pub struct vint8m1_t(i8);
impl Copy for vint8m1_t {}

#[rustc_scalable_vector(2)]
pub struct vfloat32m1_t(f32);
impl Copy for vfloat32m1_t {}

#[rustc_scalable_vector(8)]
pub struct vbool8_t(bool);
impl Copy for vbool8_t {}

#[rustc_scalable_vector(2)]
pub struct vbool32_t(bool);
impl Copy for vbool32_t {}

#[inline]
#[target_feature(enable = "v")]
#[rustc_legacy_const_generics(2)]
pub unsafe fn vaadd_vv_i8m1<const VXRM: usize>(
    vs2: vint8m1_t,
    vs1: vint8m1_t,
    vl: usize,
) -> vint8m1_t {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(target_arch = "riscv32", link_name = "llvm.riscv.vaadd.nxv8i8.nxv8i8.i32")]
        #[cfg_attr(target_arch = "riscv64", link_name = "llvm.riscv.vaadd.nxv8i8.nxv8i8.i64")]
        fn f(
            passthru: vint8m1_t,
            vs2: vint8m1_t,
            vs1: vint8m1_t,
            vxrm: usize,
            vl: usize,
        ) -> vint8m1_t;
    }
    unsafe { f(vs2, vs2, vs1, VXRM, vl) }
}

#[inline]
#[target_feature(enable = "v")]
#[rustc_legacy_const_generics(3)]
pub unsafe fn vaadd_vv_i8m1_m<const VXRM: usize>(
    vm: vbool8_t,
    vs2: vint8m1_t,
    vs1: vint8m1_t,
    vl: usize,
) -> vint8m1_t {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(target_arch = "riscv32", link_name = "llvm.riscv.vaadd.mask.nxv8i8.nxv8i8.i32")]
        #[cfg_attr(target_arch = "riscv64", link_name = "llvm.riscv.vaadd.mask.nxv8i8.nxv8i8.i64")]
        fn f(
            passthru: vint8m1_t,
            vs2: vint8m1_t,
            vs1: vint8m1_t,
            mask: vbool8_t,
            vxrm: usize,
            vl: usize,
            policy: usize,
        ) -> vint8m1_t;
    }
    unsafe { f(vs2, vs2, vs1, vm, VXRM, vl, 3) }
}

#[inline]
#[target_feature(enable = "v")]
#[rustc_legacy_const_generics(2)]
pub unsafe fn vfadd_vv_f32m1_rm<const FRM: usize>(
    vs2: vfloat32m1_t,
    vs1: vfloat32m1_t,
    vl: usize,
) -> vfloat32m1_t {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(target_arch = "riscv32", link_name = "llvm.riscv.vfadd.nxv2f32.nxv2f32.i32")]
        #[cfg_attr(target_arch = "riscv64", link_name = "llvm.riscv.vfadd.nxv2f32.nxv2f32.i64")]
        fn f(
            passthru: vfloat32m1_t,
            vs2: vfloat32m1_t,
            vs1: vfloat32m1_t,
            frm: usize,
            vl: usize,
        ) -> vfloat32m1_t;
    }
    unsafe { f(vs2, vs2, vs1, FRM, vl) }
}

#[inline]
#[target_feature(enable = "v")]
#[rustc_legacy_const_generics(3)]
pub unsafe fn vfadd_vv_f32m1_rm_m<const FRM: usize>(
    vm: vbool32_t,
    vs2: vfloat32m1_t,
    vs1: vfloat32m1_t,
    vl: usize,
) -> vfloat32m1_t {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(
            target_arch = "riscv32",
            link_name = "llvm.riscv.vfadd.mask.nxv2f32.nxv2f32.i32"
        )]
        #[cfg_attr(
            target_arch = "riscv64",
            link_name = "llvm.riscv.vfadd.mask.nxv2f32.nxv2f32.i64"
        )]
        fn f(
            passthru: vfloat32m1_t,
            vs2: vfloat32m1_t,
            vs1: vfloat32m1_t,
            mask: vbool32_t,
            frm: usize,
            vl: usize,
            policy: usize,
        ) -> vfloat32m1_t;
    }
    unsafe { f(vs2, vs2, vs1, vm, FRM, vl, 3) }
}
// CHECK-LABEL: define <vscale x 8 x i8> @vaadd_unmasked
// CHECK: call <vscale x 8 x i8> @llvm.riscv.vaadd.nxv8i8.nxv8i8.i{{32|64}}(
// CHECK-SAME: <vscale x 8 x i8> %vs2, <vscale x 8 x i8> %vs2, <vscale x 8 x i8> %vs1, i{{32|64}} 3, i{{32|64}} %vl)
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vaadd_unmasked(vs2: vint8m1_t, vs1: vint8m1_t, vl: usize) -> vint8m1_t {
    unsafe { vaadd_vv_i8m1::<3>(vs2, vs1, vl) }
}

// CHECK-LABEL: define <vscale x 8 x i8> @vaadd_masked
// CHECK: call <vscale x 8 x i8> @llvm.riscv.vaadd.mask.nxv8i8.nxv8i8.i{{32|64}}(
// CHECK-SAME: <vscale x 8 x i8> %vs2, <vscale x 8 x i8> %vs2, <vscale x 8 x i8> %vs1, <vscale x 8 x i1> %vm, i{{32|64}} 2, i{{32|64}} %vl, i{{32|64}} 3)
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vaadd_masked(vm: vbool8_t, vs2: vint8m1_t, vs1: vint8m1_t, vl: usize) -> vint8m1_t {
    unsafe { vaadd_vv_i8m1_m::<2>(vm, vs2, vs1, vl) }
}

// CHECK-LABEL: define <vscale x 2 x float> @vfadd_unmasked
// CHECK: call <vscale x 2 x float> @llvm.riscv.vfadd.nxv2f32.nxv2f32.i{{32|64}}(
// CHECK-SAME: <vscale x 2 x float> %vs2, <vscale x 2 x float> %vs2, <vscale x 2 x float> %vs1, i{{32|64}} 4, i{{32|64}} %vl)
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vfadd_unmasked(vs2: vfloat32m1_t, vs1: vfloat32m1_t, vl: usize) -> vfloat32m1_t {
    unsafe { vfadd_vv_f32m1_rm::<4>(vs2, vs1, vl) }
}

// CHECK-LABEL: define <vscale x 2 x float> @vfadd_masked
// CHECK: call <vscale x 2 x float> @llvm.riscv.vfadd.mask.nxv2f32.nxv2f32.i{{32|64}}(
// CHECK-SAME: <vscale x 2 x float> %vs2, <vscale x 2 x float> %vs2, <vscale x 2 x float> %vs1, <vscale x 2 x i1> %vm, i{{32|64}} 1, i{{32|64}} %vl, i{{32|64}} 3)
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vfadd_masked(
    vm: vbool32_t,
    vs2: vfloat32m1_t,
    vs1: vfloat32m1_t,
    vl: usize,
) -> vfloat32m1_t {
    unsafe { vfadd_vv_f32m1_rm_m::<1>(vm, vs2, vs1, vl) }
}
