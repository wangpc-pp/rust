//@ add-minicore
//@ check-fail
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] needs-llvm-components: riscv
//@ ignore-backends: gcc

#![crate_type = "lib"]
#![feature(asm_experimental_reg, no_core, riscv_target_feature, rustc_attrs)]
#![no_core]
#![allow(internal_features, non_camel_case_types, unused_assignments, unused_variables)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(1)]
pub struct vint16mf4_t(i16);
impl Copy for vint16mf4_t {}

#[rustc_scalable_vector(4)]
pub struct vint16m1_t(i16);
impl Copy for vint16m1_t {}

#[rustc_scalable_vector(8)]
pub struct vint16m2_t(i16);
impl Copy for vint16m2_t {}

#[rustc_scalable_vector(1)]
pub struct vint64m1_t(i64);
impl Copy for vint64m1_t {}

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

#[rustc_scalable_vector(8)]
pub struct vbool8_t(bool);
impl Copy for vbool8_t {}

#[rustc_scalable_vector]
pub struct vint16m1x2_t(vint16m1_t, vint16m1_t);
impl Copy for vint16m1x2_t {}

#[target_feature(enable = "v")]
unsafe fn accepts_vreg_types(
    vf4: vint16mf4_t,
    vf2: vint16m2_t,
    vi64: vint64m1_t,
    ff32: vfloat32m1_t,
    ff64: vfloat64m1_t,
    vm64: vbool64_t,
    vm32: vbool32_t,
    vm8: vbool8_t,
) {
    unsafe {
        asm!("vmv.v.v {0}, {0}", in(vreg) vf2);
        asm!("vmv.v.v {0}, {0}", in(vreg) vf4);
        asm!("vmv.v.v {0}, {0}", in(vreg) vi64);
        asm!("vmv.v.v {0}, {0}", in(vreg) ff32);
        asm!("vmv.v.v {0}, {0}", in(vreg) ff64);
        asm!("vmand.mm {0}, {0}, {0}", in(vreg) vm64);
        asm!("vmand.mm {0}, {0}, {0}", in(vreg) vm32);
        asm!("vmand.mm {0}, {0}, {0}", in(vreg) vm8);
        asm!("vmv.v.v v8, v8", in("v8") vf2);

        let late_mask: vbool32_t;
        asm!("vmset.m {0}", lateout(vreg) late_mask);

        let mut inlate = ff32;
        asm!("vmv.v.v {0}, {0}", inlateout(vreg) inlate);
    }
}

fn rejects_without_v(vf2: vint16m2_t, vm: vbool32_t) {
    unsafe {
        asm!("vmv.v.v {0}, {0}", in(vreg) vf2);
        //~^ ERROR register class `vreg` requires the `v` target feature
        asm!("vmand.mm {0}, {0}, {0}", in(vreg) vm);
        //~^ ERROR register class `vreg` requires the `v` target feature
        asm!("vmv.v.v v0, v0", in("v0") vf2);
        //~^ ERROR register class `vreg` requires the `v` target feature

        let late: vbool32_t;
        asm!("vmset.m {0}", lateout(vreg) late);
        //~^ ERROR register class `vreg` requires the `v` target feature
    }
}

#[target_feature(enable = "v")]
unsafe fn rejects_tuple(tuple: vint16m1x2_t) {
    unsafe {
        asm!("/* {0} */", in(vreg) tuple);
        //~^ ERROR cannot use value of type `vint16m1x2_t` for inline assembly
    }
}
