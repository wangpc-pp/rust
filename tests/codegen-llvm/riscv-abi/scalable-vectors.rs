//@ add-minicore
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -Ctarget-feature=+v
//@ compile-flags: -C no-prepopulate-passes
//@ compile-flags: -Zverify-llvm-ir

#![crate_type = "rlib"]
#![feature(decl_macro, eii_internals, explicit_tail_calls, extern_item_impls, no_core, rustc_attrs)]
#![no_core]
#![allow(
    dead_code,
    incomplete_features,
    internal_features,
    improper_ctypes_definitions,
    non_camel_case_types
)]

extern crate minicore;
use minicore::*;

// The public `eii` macro normally expands through core's prelude. This no-core
// test declares the built-in helper locally so it can exercise the alias shim.
#[rustc_builtin_macro]
macro eii_declaration($item: item) {}

#[rustc_scalable_vector(2)]
pub struct vint32m1_t(i32);
impl Copy for vint32m1_t {}

#[rustc_scalable_vector]
pub struct vint32m1x2_t(vint32m1_t, vint32m1_t);
impl Copy for vint32m1x2_t {}

#[rustc_scalable_vector]
pub struct vint32m1x8_t(
    vint32m1_t,
    vint32m1_t,
    vint32m1_t,
    vint32m1_t,
    vint32m1_t,
    vint32m1_t,
    vint32m1_t,
    vint32m1_t,
);
impl Copy for vint32m1x8_t {}

#[rustc_scalable_vector(1)]
pub struct vint8mf8_t(i8);
impl Copy for vint8mf8_t {}

#[rustc_scalable_vector]
pub struct vint8mf8x8_t(
    vint8mf8_t,
    vint8mf8_t,
    vint8mf8_t,
    vint8mf8_t,
    vint8mf8_t,
    vint8mf8_t,
    vint8mf8_t,
    vint8mf8_t,
);
impl Copy for vint8mf8x8_t {}

#[rustc_scalable_vector(16)]
pub struct vint8m2_t(i8);
impl Copy for vint8m2_t {}

#[rustc_scalable_vector]
pub struct vint8m2x4_t(vint8m2_t, vint8m2_t, vint8m2_t, vint8m2_t);
impl Copy for vint8m2x4_t {}

#[rustc_scalable_vector(32)]
pub struct vint8m4_t(i8);
impl Copy for vint8m4_t {}

#[rustc_scalable_vector]
pub struct vint8m4x2_t(vint8m4_t, vint8m4_t);
impl Copy for vint8m4x2_t {}

#[rustc_scalable_vector(2)]
pub struct vfloat32m1_t(f32);
impl Copy for vfloat32m1_t {}

#[rustc_scalable_vector]
pub struct vfloat32m1x3_t(vfloat32m1_t, vfloat32m1_t, vfloat32m1_t);
impl Copy for vfloat32m1x3_t {}

#[rustc_scalable_vector(8)]
pub struct vbool8_t(bool);
impl Copy for vbool8_t {}

#[rustc_scalable_vector]
pub struct vbool8x2_t(vbool8_t, vbool8_t);
impl Copy for vbool8x2_t {}

// CHECK-LABEL: define target("riscv.vector.tuple", <vscale x 8 x i8>, 2) @c_tuple(
// CHECK-SAME: target("riscv.vector.tuple", <vscale x 8 x i8>, 2) %{{[0-9]+}})
// CHECK-NOT: riscv_vector_cc
// CHECK: call <vscale x 2 x i32> @llvm.riscv.tuple.extract{{.*}}i32 0)
// CHECK: call <vscale x 2 x i32> @llvm.riscv.tuple.extract{{.*}}i32 1)
// CHECK: call target("riscv.vector.tuple", <vscale x 8 x i8>, 2) @llvm.riscv.tuple.insert{{.*}}i32 0)
// CHECK: call target("riscv.vector.tuple", <vscale x 8 x i8>, 2) @llvm.riscv.tuple.insert{{.*}}i32 1)
// CHECK: ret target("riscv.vector.tuple", <vscale x 8 x i8>, 2)
#[unsafe(no_mangle)]
pub extern "C" fn c_tuple(x: vint32m1x2_t) -> vint32m1x2_t {
    x
}

// CHECK-LABEL: define target("riscv.vector.tuple", <vscale x 8 x i8>, 8) @c_unwind_tuple8(
// CHECK-SAME: target("riscv.vector.tuple", <vscale x 8 x i8>, 8) %{{[0-9]+}})
// CHECK-NOT: riscv_vector_cc
// CHECK: call <vscale x 2 x i32> @llvm.riscv.tuple.extract{{.*}}i32 7)
// CHECK: call target("riscv.vector.tuple", <vscale x 8 x i8>, 8) @llvm.riscv.tuple.insert{{.*}}i32 7)
// CHECK: ret target("riscv.vector.tuple", <vscale x 8 x i8>, 8)
#[unsafe(no_mangle)]
pub extern "C-unwind" fn c_unwind_tuple8(x: vint32m1x8_t) -> vint32m1x8_t {
    x
}

// The smallest fractional LMUL is still a legal tuple field, including at NF=8.
// CHECK-LABEL: define target("riscv.vector.tuple", <vscale x 1 x i8>, 8) @c_fractional_tuple8(
// CHECK-SAME: target("riscv.vector.tuple", <vscale x 1 x i8>, 8) %{{[0-9]+}})
// CHECK: call <vscale x 1 x i8> @llvm.riscv.tuple.extract{{.*}}i32 7)
// CHECK: call target("riscv.vector.tuple", <vscale x 1 x i8>, 8) @llvm.riscv.tuple.insert{{.*}}i32 7)
// CHECK: ret target("riscv.vector.tuple", <vscale x 1 x i8>, 8)
#[unsafe(no_mangle)]
pub extern "C" fn c_fractional_tuple8(x: vint8mf8x8_t) -> vint8mf8x8_t {
    x
}

// An LMUL=2 field may use up to four tuple fields.
// CHECK-LABEL: define target("riscv.vector.tuple", <vscale x 16 x i8>, 4) @c_m2_tuple4(
// CHECK-SAME: target("riscv.vector.tuple", <vscale x 16 x i8>, 4) %{{[0-9]+}})
// CHECK: call <vscale x 16 x i8> @llvm.riscv.tuple.extract{{.*}}i32 3)
// CHECK: call target("riscv.vector.tuple", <vscale x 16 x i8>, 4) @llvm.riscv.tuple.insert{{.*}}i32 3)
// CHECK: ret target("riscv.vector.tuple", <vscale x 16 x i8>, 4)
#[unsafe(no_mangle)]
pub extern "C" fn c_m2_tuple4(x: vint8m2x4_t) -> vint8m2x4_t {
    x
}

// An LMUL=4 field is legal only through NF=2.
// CHECK-LABEL: define target("riscv.vector.tuple", <vscale x 32 x i8>, 2) @c_m4_tuple2(
// CHECK-SAME: target("riscv.vector.tuple", <vscale x 32 x i8>, 2) %{{[0-9]+}})
// CHECK: call <vscale x 32 x i8> @llvm.riscv.tuple.extract{{.*}}i32 1)
// CHECK: call target("riscv.vector.tuple", <vscale x 32 x i8>, 2) @llvm.riscv.tuple.insert{{.*}}i32 1)
// CHECK: ret target("riscv.vector.tuple", <vscale x 32 x i8>, 2)
#[unsafe(no_mangle)]
pub extern "C" fn c_m4_tuple2(x: vint8m4x2_t) -> vint8m4x2_t {
    x
}

// Floating-point tuple fields use the same target tuple representation.
// CHECK-LABEL: define target("riscv.vector.tuple", <vscale x 8 x i8>, 3) @c_float_tuple3(
// CHECK-SAME: target("riscv.vector.tuple", <vscale x 8 x i8>, 3) %{{[0-9]+}})
// CHECK: call <vscale x 2 x float> @llvm.riscv.tuple.extract{{.*}}i32 2)
// CHECK: call target("riscv.vector.tuple", <vscale x 8 x i8>, 3) @llvm.riscv.tuple.insert{{.*}}i32 2)
// CHECK: ret target("riscv.vector.tuple", <vscale x 8 x i8>, 3)
#[unsafe(no_mangle)]
pub extern "C" fn c_float_tuple3(x: vfloat32m1x3_t) -> vfloat32m1x3_t {
    x
}

// CHECK-LABEL: define <vscale x 2 x i32> @c_vector(
// CHECK-SAME: <vscale x 2 x i32> %x)
#[unsafe(no_mangle)]
pub extern "C" fn c_vector(x: vint32m1_t) -> vint32m1_t {
    x
}

// rv32-LABEL: define { <vscale x 2 x i32>, <vscale x 2 x i32> } @rust_tuple(
// rv32-SAME: { <vscale x 2 x i32>, <vscale x 2 x i32> } %x)
// rv32: ret { <vscale x 2 x i32>, <vscale x 2 x i32> } %x
// rv64-LABEL: define { <vscale x 2 x i32>, <vscale x 2 x i32> } @rust_tuple(
// rv64-SAME: { <vscale x 2 x i32>, <vscale x 2 x i32> } %x)
// rv64: ret { <vscale x 2 x i32>, <vscale x 2 x i32> } %x
#[unsafe(no_mangle)]
pub fn rust_tuple(x: vint32m1x2_t) -> vint32m1x2_t {
    x
}

// CHECK-LABEL: define <vscale x 2 x i32> @rust_vector(
// CHECK-SAME: <vscale x 2 x i32> %x)
#[unsafe(no_mangle)]
pub fn rust_vector(x: vint32m1_t) -> vint32m1_t {
    x
}

// Mask tuples are not legal at the C boundary, but remain available to the Rust ABI.
// CHECK-LABEL: define { <vscale x 8 x i1>, <vscale x 8 x i1> } @rust_mask_tuple(
// CHECK-SAME: { <vscale x 8 x i1>, <vscale x 8 x i1> } %x)
// CHECK-NOT: target("riscv.vector.tuple"
// CHECK-NOT: @llvm.riscv.tuple.
// CHECK: ret { <vscale x 8 x i1>, <vscale x 8 x i1> } %x
#[unsafe(no_mangle)]
pub fn rust_mask_tuple(x: vbool8x2_t) -> vbool8x2_t {
    x
}

unsafe extern "C" {
    safe fn external_c_tuple(x: vint32m1x2_t) -> vint32m1x2_t;
}

// CHECK-LABEL: define target("riscv.vector.tuple", <vscale x 8 x i8>, 2) @call_external_c_tuple(
// CHECK: call target("riscv.vector.tuple", <vscale x 8 x i8>, 2) @external_c_tuple(
// CHECK: ret target("riscv.vector.tuple", <vscale x 8 x i8>, 2)
#[unsafe(no_mangle)]
pub extern "C" fn call_external_c_tuple(x: vint32m1x2_t) -> vint32m1x2_t {
    external_c_tuple(x)
}

// CHECK-LABEL: define target("riscv.vector.tuple", <vscale x 8 x i8>, 2) @tail_c_tuple(
// CHECK: musttail call target("riscv.vector.tuple", <vscale x 8 x i8>, 2) @external_c_tuple(
// CHECK-NEXT: ret target("riscv.vector.tuple", <vscale x 8 x i8>, 2)
#[unsafe(no_mangle)]
pub extern "C" fn tail_c_tuple(x: vint32m1x2_t) -> vint32m1x2_t {
    become external_c_tuple(x)
}

// `#![no_core]` has no prelude from which to import the public `eii` macro, so
// use its built-in declaration form directly.
#[eii_declaration(tuple_alias)]
#[rustc_builtin_macro(eii_shared_macro)]
macro tuple_alias_eii() {}

unsafe extern "C" {
    safe fn tuple_alias(x: vint32m1x2_t) -> vint32m1x2_t;
}

// CHECK-LABEL: define target("riscv.vector.tuple", <vscale x 8 x i8>, 2) @tuple_alias(
// CHECK: call target("riscv.vector.tuple", <vscale x 8 x i8>, 2) @{{.*}}tuple_alias_impl(
// CHECK: ret target("riscv.vector.tuple", <vscale x 8 x i8>, 2)
#[tuple_alias_eii]
extern "C" fn tuple_alias_impl(x: vint32m1x2_t) -> vint32m1x2_t {
    x
}
