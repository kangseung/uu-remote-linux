/* Enumerate logical output geometry via xdg-output-unstable-v1. A fresh
 * wl_display connection used only at portal session setup; every failure
 * path returns -1 so callers keep the identity pointer mapping. */
#include "output_layout.h"
#include "xdg-output-unstable-v1-client-protocol.h"
#include <wayland-client.h>
#include <string.h>

#define MAX_OUTPUTS 32

struct pending { struct uu_output output; int have_position, have_size; };
struct layout {
    uint32_t output_names[MAX_OUTPUTS];
    int output_count;
    uint32_t manager_name, manager_version;
};

static void registry_global(void *data, struct wl_registry *registry, uint32_t name,
    const char *interface, uint32_t version) {
    (void)registry;
    struct layout *layout = data;
    if (!strcmp(interface, zxdg_output_manager_v1_interface.name)) {
        if (!layout->manager_name) { layout->manager_name = name; layout->manager_version = version; }
    } else if (!strcmp(interface, wl_output_interface.name)) {
        if (layout->output_count < MAX_OUTPUTS) layout->output_names[layout->output_count++] = name;
    }
}
static void registry_remove(void *data, struct wl_registry *registry, uint32_t name) {
    (void)data; (void)registry; (void)name;
}
static const struct wl_registry_listener registry_listener = {
    .global = registry_global, .global_remove = registry_remove
};

static void logical_position(void *data, struct zxdg_output_v1 *xdg_output, int32_t x, int32_t y) {
    struct pending *pending = data; (void)xdg_output;
    pending->output.x = x; pending->output.y = y; pending->have_position = 1;
}
static void logical_size(void *data, struct zxdg_output_v1 *xdg_output, int32_t width, int32_t height) {
    struct pending *pending = data; (void)xdg_output;
    pending->output.width = width; pending->output.height = height; pending->have_size = 1;
}
static void ignored_string(void *data, struct zxdg_output_v1 *xdg_output, const char *value) {
    (void)data; (void)xdg_output; (void)value;
}
static void ignored_done(void *data, struct zxdg_output_v1 *xdg_output) { (void)data; (void)xdg_output; }
static const struct zxdg_output_v1_listener xdg_listener = {
    .logical_position = logical_position, .logical_size = logical_size,
    .done = ignored_done, .name = ignored_string, .description = ignored_string
};

int uu_output_layout(struct uu_output *outputs, int capacity) {
    if (!outputs || capacity <= 0) return -1;
    struct wl_display *display = wl_display_connect(NULL);
    if (!display) return -1;
    int result = -1;
    struct wl_registry *registry = wl_display_get_registry(display);
    struct layout layout = {0};
    struct pending pending[MAX_OUTPUTS] = {0};
    struct wl_output *wl_outputs[MAX_OUTPUTS] = {0};
    struct zxdg_output_v1 *xdg_outputs[MAX_OUTPUTS] = {0};
    struct zxdg_output_manager_v1 *manager = NULL;
    if (!registry) goto done;
    wl_registry_add_listener(registry, &registry_listener, &layout);
    if (wl_display_roundtrip(display) < 0 || !layout.manager_name || !layout.output_count) goto done;
    manager = wl_registry_bind(registry, layout.manager_name, &zxdg_output_manager_v1_interface,
        layout.manager_version < 2 ? layout.manager_version : 2);
    for (int i = 0; i < layout.output_count; i++) {
        wl_outputs[i] = wl_registry_bind(registry, layout.output_names[i], &wl_output_interface, 1);
        xdg_outputs[i] = zxdg_output_manager_v1_get_xdg_output(manager, wl_outputs[i]);
        if (!xdg_outputs[i]) goto done;
        zxdg_output_v1_add_listener(xdg_outputs[i], &xdg_listener, &pending[i]);
    }
    /* Geometry events answer the get_xdg_output requests issued above. */
    if (wl_display_roundtrip(display) < 0) goto done;
    result = 0;
    for (int i = 0; i < layout.output_count && result < capacity; i++)
        if (pending[i].have_position && pending[i].have_size &&
            pending[i].output.width > 0 && pending[i].output.height > 0)
            outputs[result++] = pending[i].output;
done:
    for (int i = 0; i < layout.output_count; i++) {
        if (xdg_outputs[i]) zxdg_output_v1_destroy(xdg_outputs[i]);
        if (wl_outputs[i]) wl_output_destroy(wl_outputs[i]);
    }
    if (manager) zxdg_output_manager_v1_destroy(manager);
    if (registry) wl_registry_destroy(registry);
    wl_display_disconnect(display);
    return result;
}
