//@ add-minicore
//@ needs-llvm-components: riscv
//@ min-llvm-version: 22
//@ compile-flags: --target=riscv64gc-unknown-linux-gnu

#![feature(no_core, riscv_target_feature)]
#![no_core]
#![crate_type = "lib"]

extern crate minicore;

#[unsafe(no_mangle)]
#[target_feature(enable = "zvfbfa")]
pub unsafe fn zvfbfa() {}

#[unsafe(no_mangle)]
#[target_feature(enable = "zvfbfmin")]
pub unsafe fn zvfbfmin() {}

#[unsafe(no_mangle)]
#[target_feature(enable = "zvfbfwma")]
pub unsafe fn zvfbfwma() {}

#[unsafe(no_mangle)]
#[target_feature(enable = "zvfofp8min")]
pub unsafe fn zvfofp8min() {}

// CHECK-LABEL: define{{.*}} @zvfbfa()
// CHECK-SAME: #[[ZVFBFA_ATTR:[0-9]+]]
// CHECK-LABEL: define{{.*}} @zvfbfmin()
// CHECK-SAME: #[[ZVFBFMIN_ATTR:[0-9]+]]
// CHECK-LABEL: define{{.*}} @zvfbfwma()
// CHECK-SAME: #[[ZVFBFWMA_ATTR:[0-9]+]]
// CHECK-LABEL: define{{.*}} @zvfofp8min()
// CHECK-SAME: #[[ZVFOFP8MIN_ATTR:[0-9]+]]
// CHECK: attributes #[[ZVFBFA_ATTR]]
// CHECK-SAME: "target-features"="{{[^"]*}}+zfbfmin{{[^"]*}}+zve32f{{[^"]*}}+experimental-zvfbfa
// CHECK: attributes #[[ZVFBFMIN_ATTR]]
// CHECK-SAME: "target-features"="{{[^"]*}}+zve32f{{[^"]*}}+zvfbfmin
// CHECK: attributes #[[ZVFBFWMA_ATTR]]
// CHECK-SAME: "target-features"="{{[^"]*}}+zfbfmin{{[^"]*}}+zve32f{{[^"]*}}+zvfbfmin{{[^"]*}}+zvfbfwma
// CHECK: attributes #[[ZVFOFP8MIN_ATTR]]
// CHECK-SAME: "target-features"="{{[^"]*}}+zve32f{{[^"]*}}+experimental-zvfofp8min
