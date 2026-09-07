//@ add-minicore
//@ build-fail
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv

#![crate_type = "lib"]
#![feature(no_core, riscv_target_feature, rustc_attrs)]
#![no_core]
#![allow(improper_ctypes, improper_ctypes_definitions, internal_features)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(8)]
pub struct Vbool8(bool);

#[rustc_scalable_vector]
pub struct Vbool8x2(Vbool8, Vbool8);

type InvalidFnPointer = extern "C" fn(Vbool8x2);

#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn call_invalid_fn_pointer(function: InvalidFnPointer, x: Vbool8x2) {
    function(x);
    //~^ ERROR this function call uses scalable vector type `Vbool8x2` which has no supported RISC-V C ABI
}
