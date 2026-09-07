//@ add-minicore
//@ assembly-output: emit-asm
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ min-llvm-version: 22
//@ min-system-llvm-version: 23
//@ compile-flags: -Ctarget-feature=+v,+zvdot4a8i -Copt-level=3 -Zverify-llvm-ir
//@ compile-flags: -Zcodegen-source-order

#![crate_type = "lib"]
#![feature(link_llvm_intrinsics, no_core, riscv_target_feature, rustc_attrs)]
#![no_core]
#![allow(dead_code, improper_ctypes, internal_features, non_camel_case_types)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(2)]
pub struct vint32m1_t(i32);
impl Copy for vint32m1_t {}

#[rustc_scalable_vector(2)]
pub struct vuint32m1_t(u32);
impl Copy for vuint32m1_t {}

#[rustc_scalable_vector(2)]
pub struct vbool32_t(bool);
impl Copy for vbool32_t {}

macro_rules! declare_intrinsic {
    ($name:ident, $link:literal, ($($arg:ident: $arg_ty:ty),* $(,)?) -> $ret:ty) => {
        unsafe extern "llvm-intrinsic" {
            #[cfg_attr(target_arch = "riscv32", link_name = concat!($link, ".i32"))]
            #[cfg_attr(target_arch = "riscv64", link_name = concat!($link, ".i64"))]
            fn $name($($arg: $arg_ty),*) -> $ret;
        }
    };
}

declare_intrinsic!(
    vdot4a_vv,
    "llvm.riscv.vdot4a.nxv2i32.nxv2i32",
    (vd: vint32m1_t, vs2: vuint32m1_t, vs1: vuint32m1_t, vl: usize, policy: usize)
        -> vint32m1_t
);
declare_intrinsic!(
    vdot4a_vv_mask,
    "llvm.riscv.vdot4a.mask.nxv2i32.nxv2i32",
    (
        vd: vint32m1_t,
        vs2: vuint32m1_t,
        vs1: vuint32m1_t,
        vm: vbool32_t,
        vl: usize,
        policy: usize,
    ) -> vint32m1_t
);
declare_intrinsic!(
    vdot4a_vx,
    "llvm.riscv.vdot4a.nxv2i32.i32",
    (vd: vint32m1_t, vs2: vuint32m1_t, rs1: u32, vl: usize, policy: usize) -> vint32m1_t
);
declare_intrinsic!(
    vdot4a_vx_mask,
    "llvm.riscv.vdot4a.mask.nxv2i32.i32",
    (
        vd: vint32m1_t,
        vs2: vuint32m1_t,
        rs1: u32,
        vm: vbool32_t,
        vl: usize,
        policy: usize,
    ) -> vint32m1_t
);

declare_intrinsic!(
    vdot4asu_vv,
    "llvm.riscv.vdot4asu.nxv2i32.nxv2i32",
    (vd: vint32m1_t, vs2: vuint32m1_t, vs1: vuint32m1_t, vl: usize, policy: usize)
        -> vint32m1_t
);
declare_intrinsic!(
    vdot4asu_vv_mask,
    "llvm.riscv.vdot4asu.mask.nxv2i32.nxv2i32",
    (
        vd: vint32m1_t,
        vs2: vuint32m1_t,
        vs1: vuint32m1_t,
        vm: vbool32_t,
        vl: usize,
        policy: usize,
    ) -> vint32m1_t
);
declare_intrinsic!(
    vdot4asu_vx,
    "llvm.riscv.vdot4asu.nxv2i32.i32",
    (vd: vint32m1_t, vs2: vuint32m1_t, rs1: u32, vl: usize, policy: usize) -> vint32m1_t
);
declare_intrinsic!(
    vdot4asu_vx_mask,
    "llvm.riscv.vdot4asu.mask.nxv2i32.i32",
    (
        vd: vint32m1_t,
        vs2: vuint32m1_t,
        rs1: u32,
        vm: vbool32_t,
        vl: usize,
        policy: usize,
    ) -> vint32m1_t
);

declare_intrinsic!(
    vdot4aus_vx,
    "llvm.riscv.vdot4aus.nxv2i32.i32",
    (vd: vint32m1_t, vs2: vuint32m1_t, rs1: u32, vl: usize, policy: usize) -> vint32m1_t
);
declare_intrinsic!(
    vdot4aus_vx_mask,
    "llvm.riscv.vdot4aus.mask.nxv2i32.i32",
    (
        vd: vint32m1_t,
        vs2: vuint32m1_t,
        rs1: u32,
        vm: vbool32_t,
        vl: usize,
        policy: usize,
    ) -> vint32m1_t
);

declare_intrinsic!(
    vdot4au_vv,
    "llvm.riscv.vdot4au.nxv2i32.nxv2i32",
    (vd: vuint32m1_t, vs2: vuint32m1_t, vs1: vuint32m1_t, vl: usize, policy: usize)
        -> vuint32m1_t
);
declare_intrinsic!(
    vdot4au_vv_mask,
    "llvm.riscv.vdot4au.mask.nxv2i32.nxv2i32",
    (
        vd: vuint32m1_t,
        vs2: vuint32m1_t,
        vs1: vuint32m1_t,
        vm: vbool32_t,
        vl: usize,
        policy: usize,
    ) -> vuint32m1_t
);
declare_intrinsic!(
    vdot4au_vx,
    "llvm.riscv.vdot4au.nxv2i32.i32",
    (vd: vuint32m1_t, vs2: vuint32m1_t, rs1: u32, vl: usize, policy: usize)
        -> vuint32m1_t
);
declare_intrinsic!(
    vdot4au_vx_mask,
    "llvm.riscv.vdot4au.mask.nxv2i32.i32",
    (
        vd: vuint32m1_t,
        vs2: vuint32m1_t,
        rs1: u32,
        vm: vbool32_t,
        vl: usize,
        policy: usize,
    ) -> vuint32m1_t
);

macro_rules! test_vv {
    ($name:ident, $intrinsic:ident, $acc:ty, $lhs:ty, $rhs:ty) => {
        #[unsafe(no_mangle)]
        #[target_feature(enable = "v,zvdot4a8i")]
        pub unsafe fn $name(vd: $acc, vs2: $lhs, vs1: $rhs, vl: usize) -> $acc {
            unsafe { $intrinsic(vd, vs2, vs1, vl, 3) }
        }
    };
}

macro_rules! test_vv_masked {
    ($name:ident, $intrinsic:ident, $acc:ty, $lhs:ty, $rhs:ty) => {
        #[unsafe(no_mangle)]
        #[target_feature(enable = "v,zvdot4a8i")]
        pub unsafe fn $name(vm: vbool32_t, vd: $acc, vs2: $lhs, vs1: $rhs, vl: usize) -> $acc {
            unsafe { $intrinsic(vd, vs2, vs1, vm, vl, 3) }
        }
    };
}

macro_rules! test_vx {
    ($name:ident, $intrinsic:ident, $acc:ty, $vector:ty) => {
        #[unsafe(no_mangle)]
        #[target_feature(enable = "v,zvdot4a8i")]
        pub unsafe fn $name(vd: $acc, vs2: $vector, rs1: u32, vl: usize) -> $acc {
            unsafe { $intrinsic(vd, vs2, rs1, vl, 3) }
        }
    };
}

macro_rules! test_vx_masked {
    ($name:ident, $intrinsic:ident, $acc:ty, $vector:ty) => {
        #[unsafe(no_mangle)]
        #[target_feature(enable = "v,zvdot4a8i")]
        pub unsafe fn $name(vm: vbool32_t, vd: $acc, vs2: $vector, rs1: u32, vl: usize) -> $acc {
            unsafe { $intrinsic(vd, vs2, rs1, vm, vl, 3) }
        }
    };
}

// CHECK-LABEL: test_vdot4a_vv:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4a.vv {{v[0-9]+}}, {{v[0-9]+}}, {{v[0-9]+}}{{$}}
// CHECK-NEXT: ret
test_vv!(test_vdot4a_vv, vdot4a_vv, vint32m1_t, vuint32m1_t, vuint32m1_t);

// CHECK-LABEL: test_vdot4a_vv_masked:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4a.vv {{v[0-9]+}}, {{v[0-9]+}}, {{v[0-9]+}}, v0.t{{$}}
// CHECK-NEXT: ret
test_vv_masked!(test_vdot4a_vv_masked, vdot4a_vv_mask, vint32m1_t, vuint32m1_t, vuint32m1_t);

// CHECK-LABEL: test_vdot4a_vx:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4a.vx {{v[0-9]+}}, {{v[0-9]+}}, {{[a-z][a-z0-9]*}}{{$}}
// CHECK-NEXT: ret
test_vx!(test_vdot4a_vx, vdot4a_vx, vint32m1_t, vuint32m1_t);

// CHECK-LABEL: test_vdot4a_vx_masked:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4a.vx {{v[0-9]+}}, {{v[0-9]+}}, {{[a-z][a-z0-9]*}}, v0.t{{$}}
// CHECK-NEXT: ret
test_vx_masked!(test_vdot4a_vx_masked, vdot4a_vx_mask, vint32m1_t, vuint32m1_t);

// CHECK-LABEL: test_vdot4asu_vv:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4asu.vv {{v[0-9]+}}, {{v[0-9]+}}, {{v[0-9]+}}{{$}}
// CHECK-NEXT: ret
test_vv!(test_vdot4asu_vv, vdot4asu_vv, vint32m1_t, vuint32m1_t, vuint32m1_t);

// CHECK-LABEL: test_vdot4asu_vv_masked:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4asu.vv {{v[0-9]+}}, {{v[0-9]+}}, {{v[0-9]+}}, v0.t{{$}}
// CHECK-NEXT: ret
test_vv_masked!(test_vdot4asu_vv_masked, vdot4asu_vv_mask, vint32m1_t, vuint32m1_t, vuint32m1_t);

// CHECK-LABEL: test_vdot4asu_vx:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4asu.vx {{v[0-9]+}}, {{v[0-9]+}}, {{[a-z][a-z0-9]*}}{{$}}
// CHECK-NEXT: ret
test_vx!(test_vdot4asu_vx, vdot4asu_vx, vint32m1_t, vuint32m1_t);

// CHECK-LABEL: test_vdot4asu_vx_masked:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4asu.vx {{v[0-9]+}}, {{v[0-9]+}}, {{[a-z][a-z0-9]*}}, v0.t{{$}}
// CHECK-NEXT: ret
test_vx_masked!(test_vdot4asu_vx_masked, vdot4asu_vx_mask, vint32m1_t, vuint32m1_t);

// The `us` family has only the vector-scalar form.
// CHECK-LABEL: test_vdot4aus_vx:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4aus.vx {{v[0-9]+}}, {{v[0-9]+}}, {{[a-z][a-z0-9]*}}{{$}}
// CHECK-NEXT: ret
test_vx!(test_vdot4aus_vx, vdot4aus_vx, vint32m1_t, vuint32m1_t);

// CHECK-LABEL: test_vdot4aus_vx_masked:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4aus.vx {{v[0-9]+}}, {{v[0-9]+}}, {{[a-z][a-z0-9]*}}, v0.t{{$}}
// CHECK-NEXT: ret
test_vx_masked!(test_vdot4aus_vx_masked, vdot4aus_vx_mask, vint32m1_t, vuint32m1_t);

// CHECK-LABEL: test_vdot4au_vv:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4au.vv {{v[0-9]+}}, {{v[0-9]+}}, {{v[0-9]+}}{{$}}
// CHECK-NEXT: ret
test_vv!(test_vdot4au_vv, vdot4au_vv, vuint32m1_t, vuint32m1_t, vuint32m1_t);

// CHECK-LABEL: test_vdot4au_vv_masked:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4au.vv {{v[0-9]+}}, {{v[0-9]+}}, {{v[0-9]+}}, v0.t{{$}}
// CHECK-NEXT: ret
test_vv_masked!(test_vdot4au_vv_masked, vdot4au_vv_mask, vuint32m1_t, vuint32m1_t, vuint32m1_t);

// CHECK-LABEL: test_vdot4au_vx:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4au.vx {{v[0-9]+}}, {{v[0-9]+}}, {{[a-z][a-z0-9]*}}{{$}}
// CHECK-NEXT: ret
test_vx!(test_vdot4au_vx, vdot4au_vx, vuint32m1_t, vuint32m1_t);

// CHECK-LABEL: test_vdot4au_vx_masked:
// CHECK: vsetvli {{.*}}e32, m1, ta, ma
// CHECK-NEXT: vdot4au.vx {{v[0-9]+}}, {{v[0-9]+}}, {{[a-z][a-z0-9]*}}, v0.t{{$}}
// CHECK-NEXT: ret
test_vx_masked!(test_vdot4au_vx_masked, vdot4au_vx_mask, vuint32m1_t, vuint32m1_t);
