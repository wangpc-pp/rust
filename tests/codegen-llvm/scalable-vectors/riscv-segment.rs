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

// The data SEW is 32 while the index EEW is 16. Both vectors have two lanes,
// making the data LMUL m1 and the index EMUL mf2.
#[rustc_scalable_vector(2)]
pub struct vuint16mf2_t(u16);
impl Copy for vuint16mf2_t {}

#[rustc_scalable_vector(2)]
pub struct vbool32_t(bool);
impl Copy for vbool32_t {}

#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vlseg<T, E>(passthru: T, ptr: *const E, vl: usize) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vlseg_mask<T, E, M, const POLICY: usize>(
    passthru: T,
    ptr: *const E,
    mask: M,
    vl: usize,
) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vlsseg<T, E>(passthru: T, ptr: *const E, stride: isize, vl: usize) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vlsseg_mask<T, E, M, const POLICY: usize>(
    passthru: T,
    ptr: *const E,
    stride: isize,
    mask: M,
    vl: usize,
) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vloxseg<T, E, I>(passthru: T, ptr: *const E, index: I, vl: usize) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vloxseg_mask<T, E, I, M, const POLICY: usize>(
    passthru: T,
    ptr: *const E,
    index: I,
    mask: M,
    vl: usize,
) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vluxseg<T, E, I>(passthru: T, ptr: *const E, index: I, vl: usize) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vluxseg_mask<T, E, I, M, const POLICY: usize>(
    passthru: T,
    ptr: *const E,
    index: I,
    mask: M,
    vl: usize,
) -> T;
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vsseg<T, E>(value: T, ptr: *mut E, vl: usize);
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vsseg_mask<T, E, M>(value: T, ptr: *mut E, mask: M, vl: usize);
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vssseg<T, E>(value: T, ptr: *mut E, stride: isize, vl: usize);
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vssseg_mask<T, E, M>(value: T, ptr: *mut E, stride: isize, mask: M, vl: usize);
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vsoxseg<T, E, I>(value: T, ptr: *mut E, index: I, vl: usize);
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vsoxseg_mask<T, E, I, M>(value: T, ptr: *mut E, index: I, mask: M, vl: usize);
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vsuxseg<T, E, I>(value: T, ptr: *mut E, index: I, vl: usize);
#[rustc_intrinsic]
#[rustc_nounwind]
unsafe fn riscv_vsuxseg_mask<T, E, I, M>(value: T, ptr: *mut E, index: I, mask: M, vl: usize);

// CHECK-LABEL: @vlseg(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: @llvm.riscv.vlseg2{{.*}}(target("riscv.vector.tuple", <vscale x 8 x i8>, 2) {{.*}}, ptr %ptr, i{{32|64}} %vl, i{{32|64}} 5)
// CHECK: @llvm.riscv.tuple.extract
// CHECK-NOT: @llvm.riscv.vlse
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vlseg(passthru: vint32m1x2_t, ptr: *const i32, vl: usize) -> vint32m1x2_t {
    unsafe { riscv_vlseg(passthru, ptr, vl) }
}

// CHECK-LABEL: @vlseg_mask(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: @llvm.riscv.vlseg2.mask{{.*}}(target("riscv.vector.tuple", <vscale x 8 x i8>, 2) {{.*}}, ptr %ptr, <vscale x 2 x i1> %mask, i{{32|64}} %vl, i{{32|64}} 3, i{{32|64}} 5)
// CHECK: @llvm.riscv.tuple.extract
// CHECK-NOT: @llvm.riscv.vlse
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vlseg_mask(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    mask: vbool32_t,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vlseg_mask::<_, _, _, 3>(passthru, ptr, mask, vl) }
}

// CHECK-LABEL: @vlsseg(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: @llvm.riscv.vlsseg2{{.*}}({{.*}}, ptr %ptr, i{{32|64}} %stride, i{{32|64}} %vl, i{{32|64}} 5)
// CHECK: @llvm.riscv.tuple.extract
// CHECK-NOT: @llvm.riscv.vlse
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vlsseg(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    stride: isize,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vlsseg(passthru, ptr, stride, vl) }
}

// CHECK-LABEL: @vlsseg_mask(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: @llvm.riscv.vlsseg2.mask{{.*}}({{.*}}, ptr %ptr, i{{32|64}} %stride, <vscale x 2 x i1> %mask, i{{32|64}} %vl, i{{32|64}} 2, i{{32|64}} 5)
// CHECK: @llvm.riscv.tuple.extract
// CHECK-NOT: @llvm.riscv.vlse
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vlsseg_mask(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    stride: isize,
    mask: vbool32_t,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vlsseg_mask::<_, _, _, 2>(passthru, ptr, stride, mask, vl) }
}

// CHECK-LABEL: @vloxseg(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: @llvm.riscv.vloxseg2{{.*}}({{.*}}, ptr %ptr, <vscale x 2 x i16> %index, i{{32|64}} %vl, i{{32|64}} 5)
// CHECK: @llvm.riscv.tuple.extract
// CHECK-NOT: @llvm.riscv.vloxei
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vloxseg(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    index: vuint16mf2_t,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vloxseg(passthru, ptr, index, vl) }
}

// CHECK-LABEL: @vloxseg_mask(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: @llvm.riscv.vloxseg2.mask{{.*}}({{.*}}, ptr %ptr, <vscale x 2 x i16> %index, <vscale x 2 x i1> %mask, i{{32|64}} %vl, i{{32|64}} 1, i{{32|64}} 5)
// CHECK: @llvm.riscv.tuple.extract
// CHECK-NOT: @llvm.riscv.vloxei
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vloxseg_mask(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    index: vuint16mf2_t,
    mask: vbool32_t,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vloxseg_mask::<_, _, _, _, 1>(passthru, ptr, index, mask, vl) }
}

// CHECK-LABEL: @vluxseg(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: @llvm.riscv.vluxseg2{{.*}}({{.*}}, ptr %ptr, <vscale x 2 x i16> %index, i{{32|64}} %vl, i{{32|64}} 5)
// CHECK: @llvm.riscv.tuple.extract
// CHECK-NOT: @llvm.riscv.vluxei
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vluxseg(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    index: vuint16mf2_t,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vluxseg(passthru, ptr, index, vl) }
}

// CHECK-LABEL: @vluxseg_mask(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: @llvm.riscv.vluxseg2.mask{{.*}}({{.*}}, ptr %ptr, <vscale x 2 x i16> %index, <vscale x 2 x i1> %mask, i{{32|64}} %vl, i{{32|64}} 0, i{{32|64}} 5)
// CHECK: @llvm.riscv.tuple.extract
// CHECK-NOT: @llvm.riscv.vluxei
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vluxseg_mask(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    index: vuint16mf2_t,
    mask: vbool32_t,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vluxseg_mask::<_, _, _, _, 0>(passthru, ptr, index, mask, vl) }
}

// CHECK-LABEL: @vsseg(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: call void @llvm.riscv.vsseg2{{.*}}({{.*}}, ptr %ptr, i{{32|64}} %vl, i{{32|64}} 5)
// CHECK-NOT: @llvm.riscv.vsse
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vsseg(value: vint32m1x2_t, ptr: *mut i32, vl: usize) {
    unsafe { riscv_vsseg(value, ptr, vl) }
}

// CHECK-LABEL: @vsseg_mask(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: call void @llvm.riscv.vsseg2.mask{{.*}}({{.*}}, ptr %ptr, <vscale x 2 x i1> %mask, i{{32|64}} %vl, i{{32|64}} 5)
// CHECK-NOT: @llvm.riscv.vsse
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vsseg_mask(value: vint32m1x2_t, ptr: *mut i32, mask: vbool32_t, vl: usize) {
    unsafe { riscv_vsseg_mask(value, ptr, mask, vl) }
}

// CHECK-LABEL: @vssseg(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: call void @llvm.riscv.vssseg2{{.*}}({{.*}}, ptr %ptr, i{{32|64}} %stride, i{{32|64}} %vl, i{{32|64}} 5)
// CHECK-NOT: @llvm.riscv.vsse
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vssseg(value: vint32m1x2_t, ptr: *mut i32, stride: isize, vl: usize) {
    unsafe { riscv_vssseg(value, ptr, stride, vl) }
}

// CHECK-LABEL: @vssseg_mask(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: call void @llvm.riscv.vssseg2.mask{{.*}}({{.*}}, ptr %ptr, i{{32|64}} %stride, <vscale x 2 x i1> %mask, i{{32|64}} %vl, i{{32|64}} 5)
// CHECK-NOT: @llvm.riscv.vsse
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vssseg_mask(
    value: vint32m1x2_t,
    ptr: *mut i32,
    stride: isize,
    mask: vbool32_t,
    vl: usize,
) {
    unsafe { riscv_vssseg_mask(value, ptr, stride, mask, vl) }
}

// CHECK-LABEL: @vsoxseg(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: call void @llvm.riscv.vsoxseg2{{.*}}({{.*}}, ptr %ptr, <vscale x 2 x i16> %index, i{{32|64}} %vl, i{{32|64}} 5)
// CHECK-NOT: @llvm.riscv.vsoxei
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vsoxseg(value: vint32m1x2_t, ptr: *mut i32, index: vuint16mf2_t, vl: usize) {
    unsafe { riscv_vsoxseg(value, ptr, index, vl) }
}

// CHECK-LABEL: @vsoxseg_mask(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: call void @llvm.riscv.vsoxseg2.mask{{.*}}({{.*}}, ptr %ptr, <vscale x 2 x i16> %index, <vscale x 2 x i1> %mask, i{{32|64}} %vl, i{{32|64}} 5)
// CHECK-NOT: @llvm.riscv.vsoxei
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vsoxseg_mask(
    value: vint32m1x2_t,
    ptr: *mut i32,
    index: vuint16mf2_t,
    mask: vbool32_t,
    vl: usize,
) {
    unsafe { riscv_vsoxseg_mask(value, ptr, index, mask, vl) }
}

// CHECK-LABEL: @vsuxseg(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: call void @llvm.riscv.vsuxseg2{{.*}}({{.*}}, ptr %ptr, <vscale x 2 x i16> %index, i{{32|64}} %vl, i{{32|64}} 5)
// CHECK-NOT: @llvm.riscv.vsuxei
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vsuxseg(value: vint32m1x2_t, ptr: *mut i32, index: vuint16mf2_t, vl: usize) {
    unsafe { riscv_vsuxseg(value, ptr, index, vl) }
}

// CHECK-LABEL: @vsuxseg_mask(
// CHECK: @llvm.riscv.tuple.insert
// CHECK: call void @llvm.riscv.vsuxseg2.mask{{.*}}({{.*}}, ptr %ptr, <vscale x 2 x i16> %index, <vscale x 2 x i1> %mask, i{{32|64}} %vl, i{{32|64}} 5)
// CHECK-NOT: @llvm.riscv.vsuxei
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vsuxseg_mask(
    value: vint32m1x2_t,
    ptr: *mut i32,
    index: vuint16mf2_t,
    mask: vbool32_t,
    vl: usize,
) {
    unsafe { riscv_vsuxseg_mask(value, ptr, index, mask, vl) }
}
