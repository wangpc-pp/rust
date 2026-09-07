//@ add-minicore
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv

#![crate_type = "lib"]
#![feature(f128, no_core, rustc_attrs)]
#![no_core]
#![allow(improper_ctypes, improper_ctypes_definitions, internal_features)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(8)]
pub struct Vbool8(bool);

#[rustc_scalable_vector]
pub struct Vbool8x2(Vbool8, Vbool8);

#[rustc_scalable_vector(64)]
pub struct Vint8m8(i8);

#[rustc_scalable_vector]
pub struct Vint8m8x2(Vint8m8, Vint8m8);

#[rustc_scalable_vector(32)]
pub struct Vint8m4(i8);

#[rustc_scalable_vector]
pub struct Vint8m4x3(Vint8m4, Vint8m4, Vint8m4);

#[rustc_scalable_vector(16)]
pub struct Vint8m2(i8);

#[rustc_scalable_vector]
pub struct Vint8m2x5(Vint8m2, Vint8m2, Vint8m2, Vint8m2, Vint8m2);

#[rustc_scalable_vector(3)]
pub struct Vint32BadLanes(i32);

#[rustc_scalable_vector(1)]
pub struct Vint128(i128);

#[rustc_scalable_vector(1)]
pub struct Vuint128(u128);

#[rustc_scalable_vector(1)]
pub struct Vfloat128(f128);

unsafe extern "C" {
    safe fn declare_mask_tuple(x: Vbool8x2);
    //~^ ERROR scalable vector type `Vbool8x2` has no supported RISC-V C ABI

    safe fn declare_bad_lanes() -> Vint32BadLanes;
    //~^ ERROR scalable vector type `Vint32BadLanes` has no supported RISC-V C ABI

    safe fn declare_i128(x: Vint128);
    //~^ ERROR scalable vector type `Vint128` has no supported RISC-V C ABI

    safe fn declare_u128(x: Vuint128);
    //~^ ERROR scalable vector type `Vuint128` has no supported RISC-V C ABI

    safe fn declare_f128(x: Vfloat128);
    //~^ ERROR scalable vector type `Vfloat128` has no supported RISC-V C ABI
}

pub extern "C" fn define_m8x2(x: Vint8m8x2) {}
//~^ ERROR scalable vector type `Vint8m8x2` has no supported RISC-V C ABI

pub extern "C-unwind" fn define_m4x3() -> Vint8m4x3 {
//~^ ERROR scalable vector type `Vint8m4x3` has no supported RISC-V C ABI
    loop {}
}

pub extern "C" fn define_m2x5(x: Vint8m2x5) {}
//~^ ERROR scalable vector type `Vint8m2x5` has no supported RISC-V C ABI

// These shapes remain valid for Rust's internal ABI.
pub fn rust_mask_tuple(x: Vbool8x2) -> Vbool8x2 {
    x
}

pub fn rust_m8x2(x: Vint8m8x2) -> Vint8m8x2 {
    x
}
