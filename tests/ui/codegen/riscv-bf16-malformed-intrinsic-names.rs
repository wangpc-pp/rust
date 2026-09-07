// ignore-tidy-linelength
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -Zverify-llvm-ir
//@ normalize-stderr: "error: aborting due to 1 previous error\n\n" -> "error: aborting due to 1 previous error\n"
//@ add-minicore
//@ build-fail
//@ ignore-backends: gcc

#![crate_type = "lib"]
#![feature(link_llvm_intrinsics, no_core)]
#![no_core]
#![allow(internal_features)]

extern crate minicore;

// LLVM accepts an overloaded intrinsic's base-name prefix when looking up its
// ID. These malformed suffixes must not reach the declaration API, where LLVM
// would assert instead of letting rustc diagnose the declaration.
unsafe extern "llvm-intrinsic" {
    #[cfg(rv64)]
    #[link_name = "llvm.riscv.vadd.nxv1bf16.nxv1bf16.i64"]
    fn non_semantic_family();
    //[rv64]~^ ERROR unknown LLVM intrinsic

    #[cfg(rv32)]
    #[link_name = "llvm.riscv.vadd.nxv1bf16.nxv1bf16.i32"]
    fn non_semantic_family();
    //[rv32]~^ ERROR unknown LLVM intrinsic
}

pub unsafe fn call_malformed() {
    unsafe { non_semantic_family() }
}
