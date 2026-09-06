/* C smoke test: header compiles standalone, layout constants match the
 * documented ABI, load + get work against the live theme.
 *   ./tests/check-c.sh
 */
#include <assert.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include <omarchy_theme.h>

int main(void) {
    /* documented layout constants */
    assert(sizeof(OmarchyTheme) == 1560);
    assert(offsetof(OmarchyTheme, colors) == 88);
    assert(offsetof(OmarchyTheme, style) == 344);
    assert(offsetof(OmarchyTheme, font_family) == 472);

    if (omarchy_theme_abi_version() != OMARCHY_ABI_VERSION) {
        fprintf(stderr, "ABI mismatch: lib=%u header=%u\n",
                omarchy_theme_abi_version(), OMARCHY_ABI_VERSION);
        return 1;
    }

    OmarchyTheme t;
    memset(&t, 0, sizeof t);
    t.abi_size = (uint32_t)sizeof t;
    int32_t rc = omarchy_theme_load(&t);
    if (rc != OMARCHY_OK) {
        printf("c-smoke: skip (no live theme, rc=%d)\n", rc);
        return 0;
    }
    if ((t.colors[OMC_ACCENT] & 0xFF) != 0xFF) {
        fprintf(stderr, "alpha should be 0xFF on a resolved accent\n");
        return 1;
    }
    if ((t.colors[OMC_ANSI(0)] >> 24) != ((t.colors[OMC_BACKGROUND] >> 24) & 0xFF)) {
        fprintf(stderr, "ANSI0 red != background red\n");
        return 1;
    }
    char buf[64];
    int32_t n = omarchy_theme_get("bg", buf, sizeof buf);
    if (n <= 0 || buf[0] != '#') {
        fprintf(stderr, "get(\"bg\") failed: %d\n", n);
        return 1;
    }
    if (omarchy_theme_changed() != 0) {
        fprintf(stderr, "changed() should be 0 right after load\n");
        return 1;
    }
    printf("c-smoke: ok (theme %s, %s, accent %08X, font %s)\n",
           t.theme_name, t.mode ? "light" : "dark", t.colors[OMC_ACCENT], t.font_family);
    return 0;
}
