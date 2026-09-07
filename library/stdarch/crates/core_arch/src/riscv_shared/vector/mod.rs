//! RISC-V Vector Extension (RVV 1.0) intrinsics.
//!
//! The scalable vector types and the intrinsic functions in this module are
//! generated from the upstream RVV intrinsic listing by `stdarch-gen-riscv`;
//! see `generated.rs`. Vector register groups are modelled as scalable vectors
//! using the internal `#[rustc_scalable_vector(N)]` attribute, mirroring the
//! AArch64 SVE implementation.
//!
//! For a data vector with element width `SEW` and register-group multiplier
//! `LMUL`, the Rust type lowers to LLVM `<vscale x N x eT>` where
//! `N = 64 * LMUL / SEW` (the minimal lane count when `vscale == 1`, i.e.
//! `VLEN == 64`). Mask types `vboolK_t` lower to `<vscale x (64 / K) x i1>`.
//!
//! All computational intrinsics take the *active vector length* (`vl`) as their
//! final argument (typed `usize`, lowered to `iXLEN`), and unmasked
//! tail-agnostic variants pass an `undefined` passthru value as their first
//! operand, matching the LLVM `llvm.riscv.*` intrinsic contract.
//!
//! Segment loads/stores use compiler intrinsics to bridge the Rust tuple
//! representation to LLVM's opaque `target("riscv.vector.tuple")` type. This
//! keeps each operation as one native segment instruction, including masked,
//! indexed, and fault-only-first variants.

#![allow(non_camel_case_types)]

// `generated.rs` uses `super::*` to pull these in.
use crate::intrinsics::{simd::*, *};

#[rustfmt::skip]
mod generated;
#[cfg(test)]
#[rustfmt::skip]
mod generated_tests;
#[rustfmt::skip]
#[unstable(feature = "stdarch_riscv_vector", issue = "114544")]
pub use self::generated::*;

// -- Manually written helpers ----------------------------------------------

/// `VSEW` immediate encoding used by `llvm.riscv.vsetvli`.
#[unstable(feature = "stdarch_riscv_vector", issue = "114544")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sew {
    /// SEW = 8
    E8 = 0,
    /// SEW = 16
    E16 = 1,
    /// SEW = 32
    E32 = 2,
    /// SEW = 64
    E64 = 3,
}

/// `VLMUL` immediate encoding used by `llvm.riscv.vsetvli`.
#[unstable(feature = "stdarch_riscv_vector", issue = "114544")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lmul {
    /// LMUL = 1
    M1 = 0,
    /// LMUL = 2
    M2 = 1,
    /// LMUL = 4
    M4 = 2,
    /// LMUL = 8
    M8 = 3,
    /// LMUL = 1/8
    Mf8 = 5,
    /// LMUL = 1/4
    Mf4 = 6,
    /// LMUL = 1/2
    Mf2 = 7,
}

/// Set the active vector length (`vsetvli`), returning the granted `vl`.
///
/// `sew` and `lmul` select the element width and register-group multiplier.
#[inline]
#[target_feature(enable = "v")]
#[unstable(feature = "stdarch_riscv_vector", issue = "114544")]
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

/// Set `vl` to the maximum for the given `sew`/`lmul` (`vsetvli x, x0, ...`).
#[inline]
#[target_feature(enable = "v")]
#[unstable(feature = "stdarch_riscv_vector", issue = "114544")]
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

#[cfg(test)]
mod tests {
    use super::*;
    use stdarch_test::simd_test;

    #[simd_test(enable = "v")]
    fn smoke_arithmetic_and_memory() {
        let mut mem = [0i32; 4];
        unsafe {
            let vl = vsetvl_e32m1(4);
            let a = vmv_v_x_i32m1(7, vl);
            let b = vmv_v_x_i32m1(11, vl);
            let sum = vadd_vv_i32m1(a, b, vl);

            vse32_v_i32m1(mem.as_mut_ptr(), sum, vl);
            let loaded = vle32_v_i32m1(mem.as_ptr(), vl);
            let _ = vmv_x_s_i32m1_i32(loaded);
        }
    }

    #[simd_test(enable = "v")]
    fn smoke_mask_and_pseudo() {
        unsafe {
            let vl = vsetvlmax_e32m1();
            let mask = vmset_m_b32(vl);
            let merged = vmand_mm_b32(mask, mask, vl);
            let count = vcpop_m_b32(merged, vl);
            assert!(count <= vl);

            let a = vundefined_i32m1();
            let b = vreinterpret_v_i32m1_u32m1(a);
            let _ = b;

            let pair = vcreate_v_i32m1_i32m2(a, a);
            let lane = vget_v_i32m2_i32m1(pair, 0);
            let _ = vset_v_i32m1_i32m2(pair, 1, lane);
        }
    }
}
