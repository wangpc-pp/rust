//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ add-minicore
//@ compile-flags: -Copt-level=3 -Ctarget-feature=+v

#![crate_type = "lib"]
#![feature(intrinsics, no_core, repr_simd)]
#![allow(internal_features)]
#![no_core]

extern crate minicore;

#[rustc_intrinsic]
unsafe fn simd_eq<T, U>(x: T, y: T) -> U;

#[rustc_intrinsic]
unsafe fn simd_and<T>(x: T, y: T) -> T;

#[rustc_intrinsic]
unsafe fn simd_bitmask<T, U>(x: T) -> U;

#[repr(simd)]
pub struct U8x16([u8; 16]);

#[repr(simd)]
pub struct I8x16([i8; 16]);

// This is the vector probe used by `str::contains` for short needles. Keep it here so this test
// remains independent of the target features used to build the compiletest sysroot.
//
// CHECK-LABEL: define{{.*}} i16 @str_contains_probe(
// CHECK: icmp eq <16 x i8>
// CHECK: icmp eq <16 x i8>
// CHECK: and <16 x i1>
// CHECK: bitcast <16 x i1> {{%.*}} to i16
#[unsafe(no_mangle)]
pub unsafe fn str_contains_probe(
    first: U8x16,
    second: U8x16,
    first_probe: U8x16,
    second_probe: U8x16,
) -> u16 {
    let first_match: I8x16 = unsafe { simd_eq(first, first_probe) };
    let second_match: I8x16 = unsafe { simd_eq(second, second_probe) };
    let both = unsafe { simd_and(first_match, second_match) };
    unsafe { simd_bitmask(both) }
}
