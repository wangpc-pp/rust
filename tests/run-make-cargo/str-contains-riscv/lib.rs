#![no_std]

#[cfg(not(target_feature = "v"))]
compile_error!("the riscv64a23 baseline must enable the V extension");

// CHECK-LABEL: str_contains_public:
// CHECK: vmseq.v{{[vx]}}
// CHECK: vmseq.v{{[vx]}}
// CHECK: vmand.mm
// CHECK: .size str_contains_public
#[unsafe(no_mangle)]
#[inline(never)]
pub fn str_contains_public(haystack: &str, needle: &str) -> bool {
    haystack.contains(needle)
}
