//@ add-minicore
//@ assembly-output: emit-asm
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -Ctarget-feature=+v -Zmerge-functions=disabled

#![crate_type = "lib"]
#![feature(no_core, rustc_attrs)]
#![no_core]
#![allow(internal_features, improper_ctypes_definitions)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(2)]
pub struct Vint32m1(i32);
impl Copy for Vint32m1 {}

#[rustc_scalable_vector]
pub struct Vint32m1x2(Vint32m1, Vint32m1);
impl Copy for Vint32m1x2 {}

// CHECK: .variant_cc tuple_argument
// CHECK-NEXT: tuple_argument:
#[unsafe(no_mangle)]
pub extern "C" fn tuple_argument(_x: Vint32m1x2) {}

// CHECK: .variant_cc tuple_return
// CHECK-NEXT: tuple_return:
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tuple_return(x: *const Vint32m1x2) -> Vint32m1x2 {
    unsafe { *x }
}

// CHECK: .variant_cc tuple_argument_and_return
// CHECK-NEXT: tuple_argument_and_return:
#[unsafe(no_mangle)]
pub extern "C" fn tuple_argument_and_return(x: Vint32m1x2) -> Vint32m1x2 {
    x
}

// CHECK: .variant_cc vector_argument
// CHECK-NEXT: vector_argument:
#[unsafe(no_mangle)]
pub extern "C" fn vector_argument(_x: Vint32m1) {}

// CHECK: .variant_cc vector_return
// CHECK-NEXT: vector_return:
#[unsafe(no_mangle)]
pub unsafe extern "C" fn vector_return(x: *const Vint32m1) -> Vint32m1 {
    unsafe { *x }
}
