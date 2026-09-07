#include <riscv_vector.h>
#include <stddef.h>
#include <stdint.h>

extern vint32m1_t rust_vector_identity(vint32m1_t);
extern vint32m1x2_t rust_tuple_identity(vint32m1x2_t);
extern vint32m1_t rust_calls_c_vector(vint32m1_t, int32_t, size_t);
extern vint32m1x2_t rust_calls_c_tuple(vint32m1x2_t, size_t);

vint32m1_t c_vector_add(vint32m1_t x, int32_t delta, size_t vl) {
  return __riscv_vadd_vx_i32m1(x, delta, vl);
}

vint32m1x2_t c_tuple_add(vint32m1x2_t x, size_t vl) {
  vint32m1_t x0 = __riscv_vget_v_i32m1x2_i32m1(x, 0);
  vint32m1_t x1 = __riscv_vget_v_i32m1x2_i32m1(x, 1);
  x0 = __riscv_vadd_vx_i32m1(x0, 1000, vl);
  x1 = __riscv_vadd_vx_i32m1(x1, 2000, vl);
  return __riscv_vcreate_v_i32m1x2(x0, x1);
}

static int equal(const int32_t *actual, const int32_t *expected, size_t count) {
  for (size_t i = 0; i != count; ++i)
    if (actual[i] != expected[i])
      return 0;
  return 1;
}

int main(void) {
  const size_t vl = __riscv_vsetvl_e32m1(4);
  if (vl != 4)
    return 100;

  const int32_t a[4] = {1, 2, 3, 4};
  const int32_t tuple_data[8] = {101, 201, 102, 202, 103, 203, 104, 204};
  int32_t output[8] = {0};

  vint32m1_t va = __riscv_vle32_v_i32m1(a, vl);
  vint32m1x2_t tuple = __riscv_vlseg2e32_v_i32m1x2(tuple_data, vl);

  // C -> Rust: one vector argument and return value.
  __riscv_vse32_v_i32m1(output, rust_vector_identity(va), vl);
  if (!equal(output, a, vl))
    return 1;

  // C -> Rust: a legal NF=2, LMUL=1 tuple argument and return value.
  __riscv_vsseg2e32_v_i32m1x2(output, rust_tuple_identity(tuple), vl);
  if (!equal(output, tuple_data, 2 * vl))
    return 2;

  // Rust -> C: one vector argument and return value.
  const int32_t vector_expected[4] = {8, 9, 10, 11};
  __riscv_vse32_v_i32m1(output, rust_calls_c_vector(va, 7, vl), vl);
  if (!equal(output, vector_expected, vl))
    return 3;

  // Rust -> C: the tuple is unpacked, changed independently, repacked, and
  // returned.
  const int32_t tuple_expected[8] = {1101, 2201, 1102, 2202,
                                     1103, 2203, 1104, 2204};
  __riscv_vsseg2e32_v_i32m1x2(output, rust_calls_c_tuple(tuple, vl), vl);
  if (!equal(output, tuple_expected, 2 * vl))
    return 4;

  return 0;
}
