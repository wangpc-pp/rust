//@ add-minicore
//@ needs-llvm-components: riscv
//@ min-llvm-version: 23
//@ compile-flags: --target=riscv64gc-unknown-linux-gnu

#![feature(no_core, riscv_target_feature)]
#![no_core]
#![crate_type = "lib"]

extern crate minicore;

#[unsafe(no_mangle)]
#[target_feature(enable = "zvabd")]
pub unsafe fn zvabd() {}

#[unsafe(no_mangle)]
#[target_feature(enable = "zvdot4a8i")]
pub unsafe fn zvdot4a8i() {}

#[unsafe(no_mangle)]
#[target_feature(enable = "zvzip")]
pub unsafe fn zvzip() {}

// CHECK-LABEL: define{{.*}} @zvabd()
// CHECK-SAME: #[[ZVABD_ATTR:[0-9]+]]
// CHECK-LABEL: define{{.*}} @zvdot4a8i()
// CHECK-SAME: #[[ZVDOT4A8I_ATTR:[0-9]+]]
// CHECK-LABEL: define{{.*}} @zvzip()
// CHECK-SAME: #[[ZVZIP_ATTR:[0-9]+]]
// CHECK: attributes #[[ZVABD_ATTR]]
// CHECK-SAME: "target-features"="{{[^"]*}}+experimental-zvabd
// CHECK-SAME: +zve32x
// CHECK: attributes #[[ZVDOT4A8I_ATTR]]
// CHECK-SAME: "target-features"="{{[^"]*}}+experimental-zvdot4a8i
// CHECK-SAME: +zve32x
// CHECK: attributes #[[ZVZIP_ATTR]]
// CHECK-SAME: "target-features"="{{[^"]*}}+zve32x{{[^"]*}}+experimental-zvzip
