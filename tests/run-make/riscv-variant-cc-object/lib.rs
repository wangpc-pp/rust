#![crate_type = "lib"]
#![feature(no_core, rustc_attrs)]
#![no_core]
#![allow(internal_features, improper_ctypes_definitions)]

extern crate minicore;
use minicore::*;

#[rustc_scalable_vector(2)]
pub struct Vint32m1(i32);
impl Copy for Vint32m1 {}

#[rustc_scalable_vector]
pub struct Vint32m1x2(Vint32m1, Vint32m1);
impl Copy for Vint32m1x2 {}

#[unsafe(no_mangle)]
pub extern "C" fn vector_argument(_x: Vint32m1) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn vector_return(x: *const Vint32m1) -> Vint32m1 {
    unsafe { *x }
}

#[unsafe(no_mangle)]
pub extern "C" fn tuple_argument(_x: Vint32m1x2) {}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tuple_return(x: *const Vint32m1x2) -> Vint32m1x2 {
    unsafe { *x }
}

#[unsafe(no_mangle)]
pub extern "C" fn scalar(x: usize) -> usize {
    x
}
