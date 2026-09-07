// ignore-tidy-linelength
//@ add-minicore
//@ revisions: rv32 rv64
//@ [rv32] compile-flags: --target riscv32gc-unknown-linux-gnu
//@ [rv32] needs-llvm-components: riscv
//@ [rv64] compile-flags: --target riscv64gc-unknown-linux-gnu
//@ [rv64] needs-llvm-components: riscv
//@ compile-flags: -Cdebuginfo=2 -Copt-level=0 -Ctarget-feature=+v -Zverify-llvm-ir

#![crate_type = "lib"]
#![feature(no_core, riscv_target_feature, rustc_attrs)]
#![no_core]
#![allow(internal_features, non_camel_case_types)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(8)]
pub struct vint16m2_t(i16);

#[rustc_scalable_vector(2)]
pub struct vint16mf2_t(i16);

#[rustc_scalable_vector(2)]
pub struct vint32m1_t(i32);

#[rustc_scalable_vector]
pub struct vint32m1x2_t(vint32m1_t, vint32m1_t);

#[rustc_scalable_vector(64)]
pub struct vbool1_t(bool);

#[rustc_scalable_vector(8)]
pub struct vbool8_t(bool);

#[rustc_scalable_vector(1)]
pub struct vbool64_t(bool);

#[target_feature(enable = "v")]
pub fn locals() {
    // CHECK-DAG: name: "vint16m2_t",{{.*}}, baseType: ![[I16M2:[0-9]+]]
    // CHECK-DAG: ![[I16M2]] = !DICompositeType(tag: DW_TAG_array_type, baseType: ![[I16:[0-9]+]],{{.*}}, flags: DIFlagVector, elements: ![[I16M2_ELTS:[0-9]+]])
    // CHECK-DAG: ![[I16]] = !DIBasicType(name: "i16", size: 16, encoding: DW_ATE_signed)
    // CHECK-DAG: ![[I16M2_ELTS]] = !{![[I16M2_RANGE:[0-9]+]]}
    // CHECK-DAG: ![[I16M2_RANGE]] = !DISubrange(lowerBound: 0, upperBound: !DIExpression(DW_OP_bregx, 7202, 0, DW_OP_constu, 2, DW_OP_div, DW_OP_constu, 2, DW_OP_mul, DW_OP_constu, 1, DW_OP_minus))
    let i16m2: vint16m2_t;

    // CHECK-DAG: name: "vint16mf2_t",{{.*}}, baseType: ![[I16MF2:[0-9]+]]
    // CHECK-DAG: ![[I16MF2]] = !DICompositeType(tag: DW_TAG_array_type, baseType: ![[I16]],{{.*}}, flags: DIFlagVector, elements: ![[I16MF2_ELTS:[0-9]+]])
    // CHECK-DAG: ![[I16MF2_ELTS]] = !{![[I16MF2_RANGE:[0-9]+]]}
    // CHECK-DAG: ![[I16MF2_RANGE]] = !DISubrange(lowerBound: 0, upperBound: !DIExpression(DW_OP_bregx, 7202, 0, DW_OP_constu, 2, DW_OP_div, DW_OP_constu, 2, DW_OP_div, DW_OP_constu, 1, DW_OP_minus))
    let i16mf2: vint16mf2_t;

    // CHECK-DAG: name: "vint32m1_t",{{.*}}, baseType: ![[I32M1:[0-9]+]]
    // CHECK-DAG: ![[I32M1]] = !DICompositeType(tag: DW_TAG_array_type, baseType: ![[I32:[0-9]+]],{{.*}}, flags: DIFlagVector, elements: ![[I32M1_ELTS:[0-9]+]])
    // CHECK-DAG: ![[I32]] = !DIBasicType(name: "i32", size: 32, encoding: DW_ATE_signed)
    // CHECK-DAG: ![[I32M1_ELTS]] = !{![[I32M1_RANGE:[0-9]+]]}
    // CHECK-DAG: ![[I32M1_RANGE]] = !DISubrange(lowerBound: 0, upperBound: !DIExpression(DW_OP_bregx, 7202, 0, DW_OP_constu, 4, DW_OP_div, DW_OP_constu, 1, DW_OP_mul, DW_OP_constu, 1, DW_OP_minus))
    let i32m1: vint32m1_t;

    // CHECK-DAG: name: "vint32m1x2_t",{{.*}}, baseType: ![[I32M1X2:[0-9]+]]
    // CHECK-DAG: ![[I32M1X2]] = !DICompositeType(tag: DW_TAG_array_type, baseType: ![[I32]],{{.*}}, flags: DIFlagVector, elements: ![[I32M1X2_ELTS:[0-9]+]])
    // CHECK-DAG: ![[I32M1X2_ELTS]] = !{![[I32M1X2_RANGE:[0-9]+]]}
    // CHECK-DAG: ![[I32M1X2_RANGE]] = !DISubrange(lowerBound: 0, upperBound: !DIExpression(DW_OP_bregx, 7202, 0, DW_OP_constu, 4, DW_OP_div, DW_OP_constu, 1, DW_OP_mul, DW_OP_constu, 2, DW_OP_mul, DW_OP_constu, 1, DW_OP_minus))
    let i32m1x2: vint32m1x2_t;

    // RVV masks have one bit per logical lane.
    // CHECK-DAG: name: "vbool1_t",{{.*}}, baseType: ![[BOOL1:[0-9]+]]
    // CHECK-DAG: ![[BOOL1]] = !DICompositeType(tag: DW_TAG_array_type, baseType: ![[U8:[0-9]+]],{{.*}}, flags: DIFlagVector, elements: ![[BOOL1_ELTS:[0-9]+]], bitStride: i64 1)
    // CHECK-DAG: ![[U8]] = !DIBasicType(name: "u8", size: 8, encoding: DW_ATE_unsigned)
    // CHECK-DAG: ![[BOOL1_ELTS]] = !{![[BOOL1_RANGE:[0-9]+]]}
    // CHECK-DAG: ![[BOOL1_RANGE]] = !DISubrange(lowerBound: 0, upperBound: !DIExpression(DW_OP_bregx, 7202, 0, DW_OP_constu, 64, DW_OP_mul, DW_OP_constu, 8, DW_OP_div, DW_OP_constu, 1, DW_OP_minus))
    let mask1: vbool1_t;

    // CHECK-DAG: name: "vbool8_t",{{.*}}, baseType: ![[BOOL8:[0-9]+]]
    // CHECK-DAG: ![[BOOL8]] = !DICompositeType(tag: DW_TAG_array_type, baseType: ![[U8]],{{.*}}, flags: DIFlagVector, elements: ![[BOOL8_ELTS:[0-9]+]], bitStride: i64 1)
    // CHECK-DAG: ![[BOOL8_ELTS]] = !{![[BOOL8_RANGE:[0-9]+]]}
    // CHECK-DAG: ![[BOOL8_RANGE]] = !DISubrange(lowerBound: 0, upperBound: !DIExpression(DW_OP_bregx, 7202, 0, DW_OP_constu, 8, DW_OP_mul, DW_OP_constu, 8, DW_OP_div, DW_OP_constu, 1, DW_OP_minus))
    let mask8: vbool8_t;

    // CHECK-DAG: name: "vbool64_t",{{.*}}, baseType: ![[BOOL64:[0-9]+]]
    // CHECK-DAG: ![[BOOL64]] = !DICompositeType(tag: DW_TAG_array_type, baseType: ![[U8]],{{.*}}, flags: DIFlagVector, elements: ![[BOOL64_ELTS:[0-9]+]], bitStride: i64 1)
    // CHECK-DAG: ![[BOOL64_ELTS]] = !{![[BOOL64_RANGE:[0-9]+]]}
    // CHECK-DAG: ![[BOOL64_RANGE]] = !DISubrange(lowerBound: 0, upperBound: !DIExpression(DW_OP_bregx, 7202, 0, DW_OP_constu, 1, DW_OP_mul, DW_OP_constu, 8, DW_OP_div, DW_OP_constu, 1, DW_OP_minus))
    let mask64: vbool64_t;
}
