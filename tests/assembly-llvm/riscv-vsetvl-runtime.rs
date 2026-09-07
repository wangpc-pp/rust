//@ add-minicore
//@ assembly-output: emit-asm
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -C target-feature=+v -C opt-level=3 -Zverify-llvm-ir -Zcodegen-source-order

#![crate_type = "lib"]
#![feature(link_llvm_intrinsics, no_core, riscv_target_feature)]
#![no_core]
#![allow(internal_features)]

extern crate minicore;
use minicore::*;

pub enum Sew {
    E8,
    E16,
    E32,
    E64,
}

pub enum Lmul {
    M1,
    M2,
    M4,
    M8,
    Mf8,
    Mf4,
    Mf2,
}

#[inline]
#[target_feature(enable = "v")]
pub unsafe fn vsetvl(avl: usize, sew: Sew, lmul: Lmul) -> usize {
    #[inline]
    #[target_feature(enable = "v")]
    unsafe fn vsetvl<const VSEW: usize>(avl: usize, lmul: Lmul) -> usize {
        match lmul {
            Lmul::M1 => unsafe { vsetvl_const::<VSEW, 0>(avl) },
            Lmul::M2 => unsafe { vsetvl_const::<VSEW, 1>(avl) },
            Lmul::M4 => unsafe { vsetvl_const::<VSEW, 2>(avl) },
            Lmul::M8 => unsafe { vsetvl_const::<VSEW, 3>(avl) },
            Lmul::Mf8 => unsafe { vsetvl_const::<VSEW, 5>(avl) },
            Lmul::Mf4 => unsafe { vsetvl_const::<VSEW, 6>(avl) },
            Lmul::Mf2 => unsafe { vsetvl_const::<VSEW, 7>(avl) },
        }
    }

    match sew {
        Sew::E8 => unsafe { vsetvl::<0>(avl, lmul) },
        Sew::E16 => unsafe { vsetvl::<1>(avl, lmul) },
        Sew::E32 => unsafe { vsetvl::<2>(avl, lmul) },
        Sew::E64 => unsafe { vsetvl::<3>(avl, lmul) },
    }
}

#[inline]
#[target_feature(enable = "v")]
unsafe fn vsetvl_const<const VSEW: usize, const VLMUL: usize>(avl: usize) -> usize {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(target_arch = "riscv32", link_name = "llvm.riscv.vsetvli.i32")]
        #[cfg_attr(target_arch = "riscv64", link_name = "llvm.riscv.vsetvli.i64")]
        fn _vsetvli(avl: usize, sew: usize, lmul: usize) -> usize;
    }
    unsafe { _vsetvli(avl, VSEW, VLMUL) }
}

#[inline]
#[target_feature(enable = "v")]
pub unsafe fn vsetvlmax(sew: Sew, lmul: Lmul) -> usize {
    #[inline]
    #[target_feature(enable = "v")]
    unsafe fn vsetvlmax<const VSEW: usize>(lmul: Lmul) -> usize {
        match lmul {
            Lmul::M1 => unsafe { vsetvlmax_const::<VSEW, 0>() },
            Lmul::M2 => unsafe { vsetvlmax_const::<VSEW, 1>() },
            Lmul::M4 => unsafe { vsetvlmax_const::<VSEW, 2>() },
            Lmul::M8 => unsafe { vsetvlmax_const::<VSEW, 3>() },
            Lmul::Mf8 => unsafe { vsetvlmax_const::<VSEW, 5>() },
            Lmul::Mf4 => unsafe { vsetvlmax_const::<VSEW, 6>() },
            Lmul::Mf2 => unsafe { vsetvlmax_const::<VSEW, 7>() },
        }
    }

    match sew {
        Sew::E8 => unsafe { vsetvlmax::<0>(lmul) },
        Sew::E16 => unsafe { vsetvlmax::<1>(lmul) },
        Sew::E32 => unsafe { vsetvlmax::<2>(lmul) },
        Sew::E64 => unsafe { vsetvlmax::<3>(lmul) },
    }
}

#[inline]
#[target_feature(enable = "v")]
unsafe fn vsetvlmax_const<const VSEW: usize, const VLMUL: usize>() -> usize {
    unsafe extern "llvm-intrinsic" {
        #[cfg_attr(target_arch = "riscv32", link_name = "llvm.riscv.vsetvlimax.i32")]
        #[cfg_attr(target_arch = "riscv64", link_name = "llvm.riscv.vsetvlimax.i64")]
        fn _vsetvlimax(sew: usize, lmul: usize) -> usize;
    }
    unsafe { _vsetvlimax(VSEW, VLMUL) }
}

// Both enum arguments are runtime values. LLVM's verifier rejects this test if
// either one reaches an immarg operand instead of being dispatched here.
// CHECK-LABEL: runtime_vsetvl:
// CHECK-COUNT-28: vsetvli
// CHECK-NOT: vsetvli
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn runtime_vsetvl(avl: usize, sew: Sew, lmul: Lmul) -> usize {
    unsafe { vsetvl(avl, sew, lmul) }
}

// CHECK-LABEL: runtime_vsetvlmax:
// CHECK-COUNT-28: vsetvli
// CHECK-NOT: vsetvli
// CHECK: ret
#[unsafe(no_mangle)]
#[target_feature(enable = "v")]
pub unsafe fn runtime_vsetvlmax(sew: Sew, lmul: Lmul) -> usize {
    unsafe { vsetvlmax(sew, lmul) }
}
