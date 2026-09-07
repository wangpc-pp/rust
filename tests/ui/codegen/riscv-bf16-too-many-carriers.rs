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

unsafe extern "llvm-intrinsic" {
    #[cfg(rv64)]
    #[link_name = "llvm.riscv.vsub.nxv1bf16.nxv1bf16.i64"]
    fn too_many_carriers(
        //[rv64]~^ ERROR unknown LLVM intrinsic
        a0: u16,
        a1: u16,
        a2: u16,
        a3: u16,
        a4: u16,
        a5: u16,
        a6: u16,
        a7: u16,
        a8: u16,
        a9: u16,
        a10: u16,
        a11: u16,
        a12: u16,
    );

    #[cfg(rv32)]
    #[link_name = "llvm.riscv.vsub.nxv1bf16.nxv1bf16.i32"]
    fn too_many_carriers(
        //[rv32]~^ ERROR unknown LLVM intrinsic
        a0: u16,
        a1: u16,
        a2: u16,
        a3: u16,
        a4: u16,
        a5: u16,
        a6: u16,
        a7: u16,
        a8: u16,
        a9: u16,
        a10: u16,
        a11: u16,
        a12: u16,
    );
}

pub unsafe fn call_malformed() {
    unsafe { too_many_carriers(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0) }
}
