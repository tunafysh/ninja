#ifndef NINJA_HELPERS_H
#define NINJA_HELPERS_H

/*
 * ninja_helpers.h
 *
 * Convenience helpers for the Ninja C FFI.
 *
 * Intended usage:
 *   - Use NINJA_CHECK(expr) to jump to an `error:` label on failure.
 *   - Use NINJA_CHECK_NULL(ptr) to validate pointers and jump to `error:`.
 *   - Use NINJA_SCOPED_STRING(name, expr) to auto-free strings returned by Rust
 *     when compiling with GCC/Clang; falls back to a normal `char*` otherwise.
 *
 * This header is "header-only": helper functions are static inline.
 */

#include <stdio.h>   /* fprintf, stderr */
#include <stdlib.h>  /* free (not used, but common), abort (optional) */
#include <string.h>  /* memset (optional) */

#include "ninja.h"

#ifdef __cplusplus
extern "C" {
#endif

/* -----------------------------
 * Auto-cleanup support
 * -----------------------------
 *
 * GCC/Clang support __attribute__((cleanup(fn))) which lets us run a cleanup
 * function automatically at scope exit.
 *
 * We use this for:
 *   - strings returned from Rust that must be freed with ninja_string_free().
 */

#if defined(__GNUC__) || defined(__clang__)
#define NINJA_HAS_CLEANUP 1
#else
#define NINJA_HAS_CLEANUP 0
#endif

#if NINJA_HAS_CLEANUP
#define NINJA_AUTO_FREE __attribute__((cleanup(ninja__cleanup_string_)))
#else
/* On other compilers this becomes a no-op. */
#define NINJA_AUTO_FREE
#endif

static inline void ninja__cleanup_string_(char **p) {
    if (p && *p) {
        ninja_string_free(*p);
        *p = NULL;
    }
}

/* -----------------------------
 * Error helpers
 * ----------------------------- */

/* Print the last error (if any) to stderr with an optional context prefix. */
static inline void ninja_print_last_error(const char *context) {
    if (!ninja_has_error()) return;

    const char *prefix = (context && context[0]) ? context : "ninja";
    char *err = ninja_last_error(); /* must be freed */
    if (err) {
        fprintf(stderr, "%s: %s\n", prefix, err);
        ninja_string_free(err);
    } else {
        /* Fallback: error state set but no string available */
        fprintf(stderr, "%s: (unknown error)\n", prefix);
    }
}

/*
 * Check-and-clear helper:
 *   - if an error is present, prints it and clears global error state
 *   - returns 1 if there WAS an error, 0 otherwise
 */
static inline int ninja_check_and_clear_error(const char *context) {
    if (!ninja_has_error()) return 0;
    ninja_print_last_error(context);
    ninja_clear_last_error();
    return 1;
}

/* -----------------------------
 * Convenience macros
 * ----------------------------- */

/*
 * NINJA_CHECK(expr)
 *
 * Evaluates expr (expected to return 0 on success).
 * If it fails (non-zero), jumps to `error:` label.
 *
 * Example:
 *   NINJA_CHECK(ninja_start_shuriken(mgr, "apache"));
 */
#define NINJA_CHECK(expr)                 \
    do {                                  \
        int ninja__rc = (expr);           \
        if (ninja__rc != 0) goto error;   \
    } while (0)

/*
 * NINJA_CHECK_NULL(ptr)
 *
 * Validates ptr is not NULL; if NULL, jumps to `error:`.
 * (Does not set an error itself—typically the callee already did, or you
 *  can call ninja_print_last_error() in your error block.)
 */
#define NINJA_CHECK_NULL(ptr)             \
    do {                                  \
        if ((ptr) == NULL) goto error;    \
    } while (0)

/*
 * NINJA_SCOPED_STRING(name, expr)
 *
 * Creates a scoped char* named `name` initialized from expr.
 * On GCC/Clang, it will be auto-freed with ninja_string_free() at scope exit.
 *
 * Example:
 *   {
 *     NINJA_SCOPED_STRING(list, ninja_list_shurikens_sync(mgr, NULL));
 *     if (list) puts(list);
 *   } // list freed automatically (GCC/Clang)
 */
#if NINJA_HAS_CLEANUP
#define NINJA_SCOPED_STRING(name, expr) \
    char *name NINJA_AUTO_FREE = (expr)
#else
#define NINJA_SCOPED_STRING(name, expr) \
    char *name = (expr)
#endif

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* NINJA_HELPERS_H */