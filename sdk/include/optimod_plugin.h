/**
 * OptiMod content-checker plugin ABI (v1)
 *
 * Implement these exports in a shared library (.dll / .so / .dylib).
 * OptiMod loads plugins at runtime and calls them with UTF-8 chat message
 * text only — no user id, channel, badges, or other metadata.
 *
 * Copy this header into your plugin project. Do not depend on OptiMod internals.
 */
#ifndef OPTIMOD_PLUGIN_H
#define OPTIMOD_PLUGIN_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/** Must match optimod_abi_version() in the host. Bump only on breaking ABI changes. */
#define OPTIMOD_PLUGIN_ABI_VERSION 1

/**
 * Check return codes (optimod_check / optimod_check_cstr).
 *
 * Positive: verdict for the message (host treats Pass and Flag as deliverable).
 * Negative: caller or input error — host should log and skip the plugin.
 */
#define OPTIMOD_CHECK_PASS   1
#define OPTIMOD_CHECK_FLAG   2  /* suspicious, still deliverable unless host policy says otherwise */
#define OPTIMOD_CHECK_BLOCK  0
#define OPTIMOD_ERR_NULL    -1  /* invalid null pointer */
#define OPTIMOD_ERR_UTF8    -2  /* input is not valid UTF-8 */
#define OPTIMOD_ERR_ABI     -3  /* ABI version mismatch (host-only) */

/** Return the ABI version this plugin was built against. */
uint32_t optimod_abi_version(void);

/** Stable module id (e.g. "english"). Lowercase, no spaces. */
const char *optimod_name(void);

/**
 * Check one UTF-8 message.
 *
 * @param input UTF-8 bytes (NULL allowed only when len == 0)
 * @param len   byte length of input
 * @return OPTIMOD_CHECK_* or OPTIMOD_ERR_*
 */
int32_t optimod_check(const uint8_t *input, size_t len);

/** Convenience for NUL-terminated UTF-8. Prefer optimod_check when length is known. */
int32_t optimod_check_cstr(const char *input);

#ifdef __cplusplus
}
#endif

#endif /* OPTIMOD_PLUGIN_H */
