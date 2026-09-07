//! Scalable vector compiler intrinsics.
//!
//! In this module, a "vector" is any `#[rustc_scalable_vector]`-annotated type.

/// Create a tuple of two vectors.
///
/// `SVecTup` must be a scalable vector tuple (`#[rustc_scalable_vector]`) and `SVec` must be a
/// scalable vector (`#[rustc_scalable_vector(N)]`). `SVecTup` must be a tuple of vectors of
/// type `SVec`.
///
/// Corresponds to Clang's `__builtin_sve_svcreate2*` builtins.
#[rustc_nounwind]
#[rustc_intrinsic]
pub unsafe fn sve_tuple_create2<SVec, SVecTup>(x0: SVec, x1: SVec) -> SVecTup;

/// Create a tuple of three vectors.
///
/// `SVecTup` must be a scalable vector tuple (`#[rustc_scalable_vector]`) and `SVec` must be a
/// scalable vector (`#[rustc_scalable_vector(N)]`). `SVecTup` must be a tuple of vectors of
/// type `SVec`.
///
/// Corresponds to Clang's `__builtin_sve_svcreate3*` builtins.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn sve_tuple_create3<SVec, SVecTup>(x0: SVec, x1: SVec, x2: SVec) -> SVecTup;

/// Create a tuple of four vectors.
///
/// `SVecTup` must be a scalable vector tuple (`#[rustc_scalable_vector]`) and `SVec` must be a
/// scalable vector (`#[rustc_scalable_vector(N)]`). `SVecTup` must be a tuple of vectors of
/// type `SVec`.
///
/// Corresponds to Clang's `__builtin_sve_svcreate4*` builtins.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn sve_tuple_create4<SVec, SVecTup>(x0: SVec, x1: SVec, x2: SVec, x3: SVec) -> SVecTup;

/// Get one vector from a tuple of vectors.
///
/// `SVecTup` must be a scalable vector tuple (`#[rustc_scalable_vector]`) and `SVec` must be a
/// scalable vector (`#[rustc_scalable_vector(N)]`). `SVecTup` must be a tuple of vectors of
/// type `SVec`.
///
/// Corresponds to Clang's `__builtin_sve_svget*` builtins.
///
/// # Safety
///
/// `IDX` must be in-bounds of the tuple.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn sve_tuple_get<SVecTup, SVec, const IDX: i32>(tuple: SVecTup) -> SVec;

/// Change one vector in a tuple of vectors.
///
/// `SVecTup` must be a scalable vector tuple (`#[rustc_scalable_vector]`) and `SVec` must be a
/// scalable vector (`#[rustc_scalable_vector(N)]`). `SVecTup` must be a tuple of vectors of
/// type `SVec`.
///
/// Corresponds to Clang's `__builtin_sve_svset*` builtins.
///
/// # Safety
///
/// `IDX` must be in-bounds of the tuple.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn sve_tuple_set<SVecTup, SVec, const IDX: i32>(tuple: SVecTup, x: SVec) -> SVecTup;

/// Reinterpret an LMUL=1 RISC-V data vector as a mask vector, or vice versa.
///
/// One of `Src` and `Dst` must be an integer scalable vector occupying one full
/// RISC-V vector register. The other must be a scalable boolean vector.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vreinterpret<Src, Dst>(src: Src) -> Dst;

/// Perform a RISC-V unit-stride fault-only-first vector load.
///
/// `SVec` must be a scalable vector (`#[rustc_scalable_vector(N)]`) whose element type is `Elem`.
/// The number of elements successfully loaded is written to `new_vl`.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vleff<SVec, Elem>(
    passthru: SVec,
    ptr: *const Elem,
    new_vl: *mut usize,
    vl: usize,
) -> SVec;

/// Perform a masked RISC-V unit-stride fault-only-first vector load.
///
/// `SVec` and `Mask` must be scalable vectors (`#[rustc_scalable_vector(N)]`) with the same
/// number of elements. `SVec`'s element type must be `Elem`. The number of elements successfully
/// loaded is written to `new_vl`.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vleff_mask<SVec, Elem, Mask, const POLICY: usize>(
    passthru: SVec,
    ptr: *const Elem,
    mask: Mask,
    new_vl: *mut usize,
    vl: usize,
) -> SVec;

/// Perform a RISC-V unit-stride fault-only-first segment load.
///
/// `SVecTup` must be a tuple of two to eight scalable vectors whose element type is `Elem`. The
/// number of elements successfully loaded is written to `new_vl`.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vlsegff<SVecTup, Elem>(
    passthru: SVecTup,
    ptr: *const Elem,
    new_vl: *mut usize,
    vl: usize,
) -> SVecTup;

/// Perform a masked RISC-V unit-stride fault-only-first segment load.
///
/// `SVecTup` must be a tuple of two to eight scalable vectors whose element type is `Elem`.
/// `Mask` must be a scalable boolean vector with the same number of lanes as each tuple
/// field. The number of elements successfully loaded is written to `new_vl`.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vlsegff_mask<SVecTup, Elem, Mask, const POLICY: usize>(
    passthru: SVecTup,
    ptr: *const Elem,
    mask: Mask,
    new_vl: *mut usize,
    vl: usize,
) -> SVecTup;

/// Perform a RISC-V unit-stride segment load.
///
/// `SVecTup` must be a tuple of two to eight scalable vectors whose element type is `Elem`.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vlseg<SVecTup, Elem>(passthru: SVecTup, ptr: *const Elem, vl: usize)
-> SVecTup;

/// Perform a masked RISC-V unit-stride segment load.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vlseg_mask<SVecTup, Elem, Mask, const POLICY: usize>(
    passthru: SVecTup,
    ptr: *const Elem,
    mask: Mask,
    vl: usize,
) -> SVecTup;

/// Perform a RISC-V strided segment load.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vlsseg<SVecTup, Elem>(
    passthru: SVecTup,
    ptr: *const Elem,
    stride: isize,
    vl: usize,
) -> SVecTup;

/// Perform a masked RISC-V strided segment load.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vlsseg_mask<SVecTup, Elem, Mask, const POLICY: usize>(
    passthru: SVecTup,
    ptr: *const Elem,
    stride: isize,
    mask: Mask,
    vl: usize,
) -> SVecTup;

/// Perform a RISC-V ordered indexed segment load.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vloxseg<SVecTup, Elem, Index>(
    passthru: SVecTup,
    ptr: *const Elem,
    index: Index,
    vl: usize,
) -> SVecTup;

/// Perform a masked RISC-V ordered indexed segment load.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vloxseg_mask<SVecTup, Elem, Index, Mask, const POLICY: usize>(
    passthru: SVecTup,
    ptr: *const Elem,
    index: Index,
    mask: Mask,
    vl: usize,
) -> SVecTup;

/// Perform a RISC-V unordered indexed segment load.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vluxseg<SVecTup, Elem, Index>(
    passthru: SVecTup,
    ptr: *const Elem,
    index: Index,
    vl: usize,
) -> SVecTup;

/// Perform a masked RISC-V unordered indexed segment load.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vluxseg_mask<SVecTup, Elem, Index, Mask, const POLICY: usize>(
    passthru: SVecTup,
    ptr: *const Elem,
    index: Index,
    mask: Mask,
    vl: usize,
) -> SVecTup;

/// Perform a RISC-V unit-stride segment store.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vsseg<SVecTup, Elem>(value: SVecTup, ptr: *mut Elem, vl: usize);

/// Perform a masked RISC-V unit-stride segment store.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vsseg_mask<SVecTup, Elem, Mask>(
    value: SVecTup,
    ptr: *mut Elem,
    mask: Mask,
    vl: usize,
);

/// Perform a RISC-V strided segment store.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vssseg<SVecTup, Elem>(value: SVecTup, ptr: *mut Elem, stride: isize, vl: usize);

/// Perform a masked RISC-V strided segment store.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vssseg_mask<SVecTup, Elem, Mask>(
    value: SVecTup,
    ptr: *mut Elem,
    stride: isize,
    mask: Mask,
    vl: usize,
);

/// Perform a RISC-V ordered indexed segment store.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vsoxseg<SVecTup, Elem, Index>(
    value: SVecTup,
    ptr: *mut Elem,
    index: Index,
    vl: usize,
);

/// Perform a masked RISC-V ordered indexed segment store.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vsoxseg_mask<SVecTup, Elem, Index, Mask>(
    value: SVecTup,
    ptr: *mut Elem,
    index: Index,
    mask: Mask,
    vl: usize,
);

/// Perform a RISC-V unordered indexed segment store.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vsuxseg<SVecTup, Elem, Index>(
    value: SVecTup,
    ptr: *mut Elem,
    index: Index,
    vl: usize,
);

/// Perform a masked RISC-V unordered indexed segment store.
#[rustc_intrinsic]
#[rustc_nounwind]
pub unsafe fn riscv_vsuxseg_mask<SVecTup, Elem, Index, Mask>(
    value: SVecTup,
    ptr: *mut Elem,
    index: Index,
    mask: Mask,
    vl: usize,
);
