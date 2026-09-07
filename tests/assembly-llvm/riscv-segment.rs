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

// Keep the indexed instructions' data SEW (32) different from their index EEW (16).
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

// The CHECK-NOT pairs around each expected mnemonic ensure that every wrapper contains exactly
// one segment instruction.

// CHECK-LABEL: vlseg2e32:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vlseg2e32.v
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vlseg2e32(passthru: vint32m1x2_t, ptr: *const i32, vl: usize) -> vint32m1x2_t {
    unsafe { riscv_vlseg(passthru, ptr, vl) }
}

// CHECK-LABEL: vlseg2e32_masked:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vlseg2e32.v {{.*}}v0.t{{$}}
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vlseg2e32_masked(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    mask: vbool32_t,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vlseg_mask::<_, _, _, 3>(passthru, ptr, mask, vl) }
}

// CHECK-LABEL: vlsseg2e32:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vlsseg2e32.v
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vlsseg2e32(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    stride: isize,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vlsseg(passthru, ptr, stride, vl) }
}

// CHECK-LABEL: vlsseg2e32_masked:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vlsseg2e32.v {{.*}}v0.t{{$}}
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vlsseg2e32_masked(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    stride: isize,
    mask: vbool32_t,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vlsseg_mask::<_, _, _, 3>(passthru, ptr, stride, mask, vl) }
}

// CHECK-LABEL: vloxseg2ei16_ordered:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsetvli {{.*}}e32
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vloxseg2ei16.v
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vloxseg2ei16_ordered(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    index: vuint16mf2_t,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vloxseg(passthru, ptr, index, vl) }
}

// CHECK-LABEL: vloxseg2ei16_ordered_masked:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsetvli {{.*}}e32
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vloxseg2ei16.v {{.*}}v0.t{{$}}
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vloxseg2ei16_ordered_masked(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    index: vuint16mf2_t,
    mask: vbool32_t,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vloxseg_mask::<_, _, _, _, 3>(passthru, ptr, index, mask, vl) }
}

// CHECK-LABEL: vluxseg2ei16_unordered:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsetvli {{.*}}e32
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vluxseg2ei16.v
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vluxseg2ei16_unordered(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    index: vuint16mf2_t,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vluxseg(passthru, ptr, index, vl) }
}

// CHECK-LABEL: vluxseg2ei16_unordered_masked:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsetvli {{.*}}e32
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vluxseg2ei16.v {{.*}}v0.t{{$}}
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vluxseg2ei16_unordered_masked(
    passthru: vint32m1x2_t,
    ptr: *const i32,
    index: vuint16mf2_t,
    mask: vbool32_t,
    vl: usize,
) -> vint32m1x2_t {
    unsafe { riscv_vluxseg_mask::<_, _, _, _, 3>(passthru, ptr, index, mask, vl) }
}

// CHECK-LABEL: vsseg2e32:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsseg2e32.v
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vsseg2e32(value: vint32m1x2_t, ptr: *mut i32, vl: usize) {
    unsafe { riscv_vsseg(value, ptr, vl) }
}

// CHECK-LABEL: vsseg2e32_masked:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsseg2e32.v {{.*}}v0.t{{$}}
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vsseg2e32_masked(value: vint32m1x2_t, ptr: *mut i32, mask: vbool32_t, vl: usize) {
    unsafe { riscv_vsseg_mask(value, ptr, mask, vl) }
}

// CHECK-LABEL: vssseg2e32:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vssseg2e32.v
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vssseg2e32(value: vint32m1x2_t, ptr: *mut i32, stride: isize, vl: usize) {
    unsafe { riscv_vssseg(value, ptr, stride, vl) }
}

// CHECK-LABEL: vssseg2e32_masked:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vssseg2e32.v {{.*}}v0.t{{$}}
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vssseg2e32_masked(
    value: vint32m1x2_t,
    ptr: *mut i32,
    stride: isize,
    mask: vbool32_t,
    vl: usize,
) {
    unsafe { riscv_vssseg_mask(value, ptr, stride, mask, vl) }
}

// CHECK-LABEL: vsoxseg2ei16_ordered:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsetvli {{.*}}e32
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsoxseg2ei16.v
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vsoxseg2ei16_ordered(
    value: vint32m1x2_t,
    ptr: *mut i32,
    index: vuint16mf2_t,
    vl: usize,
) {
    unsafe { riscv_vsoxseg(value, ptr, index, vl) }
}

// CHECK-LABEL: vsoxseg2ei16_ordered_masked:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsetvli {{.*}}e32
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsoxseg2ei16.v {{.*}}v0.t{{$}}
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vsoxseg2ei16_ordered_masked(
    value: vint32m1x2_t,
    ptr: *mut i32,
    index: vuint16mf2_t,
    mask: vbool32_t,
    vl: usize,
) {
    unsafe { riscv_vsoxseg_mask(value, ptr, index, mask, vl) }
}

// CHECK-LABEL: vsuxseg2ei16_unordered:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsetvli {{.*}}e32
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsuxseg2ei16.v
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vsuxseg2ei16_unordered(
    value: vint32m1x2_t,
    ptr: *mut i32,
    index: vuint16mf2_t,
    vl: usize,
) {
    unsafe { riscv_vsuxseg(value, ptr, index, vl) }
}

// CHECK-LABEL: vsuxseg2ei16_unordered_masked:
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsetvli {{.*}}e32
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: vsuxseg2ei16.v {{.*}}v0.t{{$}}
// CHECK-NOT: {{v[a-z0-9]*seg[0-9]+e[a-z0-9]+[.]v}}
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn vsuxseg2ei16_unordered_masked(
    value: vint32m1x2_t,
    ptr: *mut i32,
    index: vuint16mf2_t,
    mask: vbool32_t,
    vl: usize,
) {
    unsafe { riscv_vsuxseg_mask(value, ptr, index, mask, vl) }
}
