#ifndef UU_OUTPUT_LAYOUT_H
#define UU_OUTPUT_LAYOUT_H
#include <stdint.h>
struct uu_output { int32_t x, y, width, height; };
/* Logical output geometry via xdg-output-unstable-v1 (compositor coordinates).
 * Returns the output count, 0 when no output reported geometry, -1 when
 * enumeration is unavailable (no Wayland session or no xdg-output support). */
int uu_output_layout(struct uu_output *outputs, int capacity);
#endif
