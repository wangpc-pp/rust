//@ check-pass
//@ add-minicore
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv

#![crate_type = "lib"]
#![feature(no_core, rustc_attrs)]
#![no_core]
#![deny(improper_ctypes, improper_ctypes_definitions)]
#![allow(internal_features)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(2)]
pub struct Vint32m1(i32);

#[rustc_scalable_vector]
pub struct Vint32m1x2(Vint32m1, Vint32m1);

#[rustc_scalable_vector(8)]
pub struct Vbool8(bool);

unsafe extern "C" {
    safe fn declare_vector(x: Vint32m1) -> Vint32m1;
    safe fn declare_tuple(x: Vint32m1x2) -> Vint32m1x2;
    safe fn declare_mask(x: Vbool8) -> Vbool8;
}

pub extern "C" fn define_vector(x: Vint32m1) -> Vint32m1 {
    x
}

pub extern "C" fn define_tuple(x: Vint32m1x2) -> Vint32m1x2 {
    x
}

pub extern "C" fn define_mask(x: Vbool8) -> Vbool8 {
    x
}
