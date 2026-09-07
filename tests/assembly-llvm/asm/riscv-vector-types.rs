//@ add-minicore
//@ assembly-output: emit-asm
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -Ctarget-feature=+v,+zvfh
//@ compile-flags: -Copt-level=3 -Zmerge-functions=disabled

#![crate_type = "rlib"]
#![feature(asm_experimental_reg, f16, no_core, rustc_attrs)]
#![no_core]
#![allow(dead_code, internal_features, non_camel_case_types)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(2)]
pub struct vint32m1_t(i32);
impl Copy for vint32m1_t {}

#[rustc_scalable_vector(1)]
pub struct vint32mf2_t(i32);
impl Copy for vint32mf2_t {}

#[rustc_scalable_vector(1)]
pub struct vint16mf4_t(i16);
impl Copy for vint16mf4_t {}

#[rustc_scalable_vector(4)]
pub struct vint16m1_t(i16);
impl Copy for vint16m1_t {}

#[rustc_scalable_vector(4)]
pub struct vint32m2_t(i32);
impl Copy for vint32m2_t {}

#[rustc_scalable_vector(1)]
pub struct vint64m1_t(i64);
impl Copy for vint64m1_t {}

#[rustc_scalable_vector(64)]
pub struct vint8m8_t(i8);
impl Copy for vint8m8_t {}

#[rustc_scalable_vector(4)]
pub struct vfloat16m1_t(f16);
impl Copy for vfloat16m1_t {}

#[rustc_scalable_vector(2)]
pub struct vfloat32m1_t(f32);
impl Copy for vfloat32m1_t {}

#[rustc_scalable_vector(1)]
pub struct vfloat64m1_t(f64);
impl Copy for vfloat64m1_t {}

#[rustc_scalable_vector(1)]
pub struct vbool64_t(bool);
impl Copy for vbool64_t {}

#[rustc_scalable_vector(2)]
pub struct vbool32_t(bool);
impl Copy for vbool32_t {}

#[rustc_scalable_vector(4)]
pub struct vbool16_t(bool);
impl Copy for vbool16_t {}

#[rustc_scalable_vector(8)]
pub struct vbool8_t(bool);
impl Copy for vbool8_t {}

// CHECK-LABEL: explicit_v8:
// CHECK: #APP
// CHECK: vmv.v.v v8, v8
// CHECK: #NO_APP
// CHECK-LABEL: vreg_in:
// CHECK: #APP
// CHECK: vmv.v.v v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_inout:
// CHECK: #APP
// CHECK: vmv.v.v v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_out:
// CHECK: #APP
// CHECK: vmv.v.i v{{[0-9]+}}, 0
// CHECK: #NO_APP
// CHECK-LABEL: vreg_lateout:
// CHECK: #APP
// CHECK: vmv.v.i v{{[0-9]+}}, 0
// CHECK: #NO_APP
// CHECK-LABEL: vreg_inlateout:
// CHECK: #APP
// CHECK: vmv.v.v v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_fractional:
// CHECK: #APP
// CHECK: vmv.v.v v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_fractional_i16:
// CHECK: #APP
// CHECK: vmv.v.v v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_i16:
// CHECK: #APP
// CHECK: vmv.v.v v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_m2:
// CHECK: #APP
// CHECK: vmv.v.v v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: explicit_v8_m2:
// CHECK: #APP
// CHECK: vmv.v.v v8, v8
// CHECK: #NO_APP
// CHECK-LABEL: vreg_i64:
// CHECK: #APP
// CHECK: vmv.v.v v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_m8:
// CHECK: #APP
// CHECK: vmv.v.v v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: explicit_v24_m8:
// CHECK: #APP
// CHECK: vmv.v.v v24, v24
// CHECK: #NO_APP
// CHECK-LABEL: vreg_float:
// CHECK: #APP
// CHECK: vmv.v.v v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_f32:
// CHECK: #APP
// CHECK: vmv.v.v v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_f64:
// CHECK: #APP
// CHECK: vmv.v.v v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_mask:
// CHECK: #APP
// CHECK: vmmv.m v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_mask_m64:
// CHECK: #APP
// CHECK: vmmv.m v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_mask_m16:
// CHECK: #APP
// CHECK: vmmv.m v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_mask_m8:
// CHECK: #APP
// CHECK: vmmv.m v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_mask_lateout:
// CHECK: #APP
// CHECK: vmset.m v{{[0-9]+}}
// CHECK: #NO_APP
// CHECK-LABEL: vreg_mask_inlateout:
// CHECK: #APP
// CHECK: vmmv.m v{{[0-9]+}}, v{{[0-9]+}}
// CHECK: #NO_APP

#[unsafe(no_mangle)]
pub unsafe fn explicit_v8(mut x: vint32m1_t) -> vint32m1_t {
    unsafe {
        asm!("vmv.v.v v8, v8", inout("v8") x);
    }
    x
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_in(x: vint32m1_t) {
    unsafe {
        asm!("vmv.v.v {0}, {0}", in(vreg) x);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_inout(mut x: vint32m1_t) -> vint32m1_t {
    unsafe {
        asm!("vmv.v.v {0}, {0}", inout(vreg) x);
    }
    x
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_out() -> vint32m1_t {
    let x;
    unsafe {
        asm!("vmv.v.i {0}, 0", out(vreg) x);
    }
    x
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_lateout() -> vint16m1_t {
    let x;
    unsafe {
        asm!("vmv.v.i {0}, 0", lateout(vreg) x);
    }
    x
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_inlateout(mut x: vfloat32m1_t) -> vfloat32m1_t {
    unsafe {
        asm!("vmv.v.v {0}, {0}", inlateout(vreg) x);
    }
    x
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_fractional(x: vint32mf2_t) {
    unsafe {
        asm!("vmv.v.v {0}, {0}", in(vreg) x);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_fractional_i16(x: vint16mf4_t) {
    unsafe {
        asm!("vmv.v.v {0}, {0}", in(vreg) x);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_i16(x: vint16m1_t) {
    unsafe {
        asm!("vmv.v.v {0}, {0}", in(vreg) x);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_m2(mut x: vint32m2_t) -> vint32m2_t {
    unsafe {
        asm!("vmv.v.v {0}, {0}", inout(vreg) x);
    }
    x
}

#[unsafe(no_mangle)]
pub unsafe fn explicit_v8_m2(mut x: vint32m2_t) -> vint32m2_t {
    unsafe {
        asm!("vmv.v.v v8, v8", inout("v8") x);
    }
    x
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_i64(x: vint64m1_t) {
    unsafe {
        asm!("vmv.v.v {0}, {0}", in(vreg) x);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_m8(x: vint8m8_t) {
    unsafe {
        asm!("vmv.v.v {0}, {0}", in(vreg) x);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn explicit_v24_m8(x: vint8m8_t) {
    unsafe {
        asm!("vmv.v.v v24, v24", in("v24") x);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_float(x: vfloat16m1_t) {
    unsafe {
        asm!("vmv.v.v {0}, {0}", in(vreg) x);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_f32(x: vfloat32m1_t) {
    unsafe {
        asm!("vmv.v.v {0}, {0}", in(vreg) x);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_f64(x: vfloat64m1_t) {
    unsafe {
        asm!("vmv.v.v {0}, {0}", in(vreg) x);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_mask(mut x: vbool32_t) -> vbool32_t {
    unsafe {
        asm!("vmand.mm {0}, {0}, {0}", inout(vreg) x);
    }
    x
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_mask_m64(x: vbool64_t) {
    unsafe {
        asm!("vmand.mm {0}, {0}, {0}", in(vreg) x);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_mask_m16(x: vbool16_t) {
    unsafe {
        asm!("vmand.mm {0}, {0}, {0}", in(vreg) x);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_mask_m8(x: vbool8_t) {
    unsafe {
        asm!("vmand.mm {0}, {0}, {0}", in(vreg) x);
    }
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_mask_lateout() -> vbool32_t {
    let x;
    unsafe {
        asm!("vmset.m {0}", lateout(vreg) x);
    }
    x
}

#[unsafe(no_mangle)]
pub unsafe fn vreg_mask_inlateout(mut x: vbool16_t) -> vbool16_t {
    unsafe {
        asm!("vmand.mm {0}, {0}, {0}", inlateout(vreg) x);
    }
    x
}
