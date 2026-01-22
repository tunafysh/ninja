# Ninja FFI - Ergonomic Improvements

This document describes the ergonomic improvements made to the Ninja FFI bindings while maintaining safety and reliability.

## Overview

The FFI has been enhanced with:
1. **Simplified error handling** - Buffer-based error retrieval without allocations
2. **Convenience functions** - Simple variants that don't require `out_err` parameters
3. **Helper macros** - C macros for common patterns
4. **Count APIs** - Quick queries without JSON parsing
5. **Backward compatibility** - All existing APIs still work

## Simplified Error Handling

### New Error Functions

```c
// Check if there's a pending error (returns 1/0)
int ninja_has_error(void);

// Get error into a buffer (no malloc needed!)
int ninja_get_last_error_buf(char *buffer, size_t buffer_size);

// Clear the error state
void ninja_clear_last_error(void);
```

### Example: Buffer-based Error Handling

**Before (required malloc/free):**
```c
char *err = NULL;
if (ninja_start_shuriken_sync(mgr, "apache", &err) != 0) {
    printf("Error: %s\n", err);
    ninja_string_free(err);  // Must remember to free!
}
```

**After (stack-based, no malloc):**
```c
if (ninja_start_shuriken(mgr, "apache") != 0) {
    char err_buf[256];
    ninja_get_last_error_buf(err_buf, sizeof(err_buf));
    printf("Error: %s\n", err_buf);  // No free needed!
}
```

## Simplified Function Variants

All synchronous operations now have simple variants without the `out_err` parameter:

| Old API (still works) | New Simple API |
|----------------------|----------------|
| `ninja_start_shuriken_sync(mgr, name, &err)` | `ninja_start_shuriken(mgr, name)` |
| `ninja_stop_shuriken_sync(mgr, name, &err)` | `ninja_stop_shuriken(mgr, name)` |
| `ninja_refresh_shuriken_sync(mgr, name, &err)` | `ninja_refresh_shuriken(mgr, name)` |
| `ninja_remove_shuriken_sync(mgr, name, &err)` | `ninja_remove_shuriken(mgr, name)` |

These use the global error state, which you check with `ninja_has_error()` or `ninja_get_last_error_buf()`.

## Count API

Get the number of shurikens without parsing JSON:

```c
int count = ninja_count_shurikens(mgr);
if (count >= 0) {
    printf("Found %d shurikens\n", count);
} else {
    printf("Error: %s\n", ninja_last_error());
}
```

## Helper Macros

Include `ninja_helpers.h` for convenient macros:

### NINJA_CHECK - Automatic error checking

```c
#include "ninja_helpers.h"

int main() {
    NinjaManagerOpaque *mgr = ninja_manager_new(NULL);
    NINJA_CHECK_NULL(mgr);
    
    NINJA_CHECK(ninja_start_shuriken(mgr, "apache"));
    NINJA_CHECK(ninja_stop_shuriken(mgr, "apache"));
    
    ninja_manager_free(mgr);
    return 0;

error:
    if (mgr) ninja_manager_free(mgr);
    return 1;
}
```

### NINJA_SCOPED_STRING - Automatic cleanup (GCC/Clang)

```c
{
    NINJA_SCOPED_STRING(list, ninja_list_shurikens_sync(mgr, NULL));
    if (list) {
        printf("%s\n", list);
    }
    // Automatically freed when leaving scope!
}
```

### Helper Functions

```c
// Print error to stderr
ninja_print_last_error("Operation failed");

// Check and clear in one call
if (ninja_check_and_clear_error("My operation")) {
    // There was an error (already printed and cleared)
}
```

## Memory Management

The improvements reduce manual memory management:

1. **Buffer-based APIs** - Use stack buffers instead of heap allocations
2. **Auto-free macros** - Automatic cleanup on supported compilers
3. **Count APIs** - Avoid parsing JSON when you only need counts
4. **Global error state** - Optional alternative to per-call error strings

## Backward Compatibility

All existing APIs continue to work:
- `ninja_start_shuriken_sync()` - Still available
- `ninja_list_shurikens_sync()` - Still returns JSON
- `out_err` parameters - Still supported
- Async callbacks - Unchanged

## Migration Guide

### Level 1: Drop-in replacements
Simply replace `_sync` variants with simple variants:
```c
// Before
ninja_start_shuriken_sync(mgr, "apache", NULL);

// After  
ninja_start_shuriken(mgr, "apache");
```

### Level 2: Use buffer-based errors
Replace heap-allocated errors with stack buffers:
```c
// Before
char *err = NULL;
if (ninja_start_shuriken_sync(mgr, name, &err) != 0) {
    printf("Error: %s\n", err);
    ninja_string_free(err);
}

// After
if (ninja_start_shuriken(mgr, name) != 0) {
    char buf[256];
    ninja_get_last_error_buf(buf, sizeof(buf));
    printf("Error: %s\n", buf);
}
```

### Level 3: Use helper macros
Add `ninja_helpers.h` and use convenience macros:
```c
#include "ninja_helpers.h"

NINJA_CHECK(ninja_start_shuriken(mgr, "apache"));
NINJA_SCOPED_STRING(list, ninja_list_shurikens_sync(mgr, NULL));
```

## Safety Guarantees

All improvements maintain safety:
- ✅ No new memory leaks introduced
- ✅ No unsafe pointer operations
- ✅ Buffer overflow protection (size checking)
- ✅ Null pointer checks
- ✅ Backward compatible
- ✅ Thread-safe error storage (mutex-protected)

## Examples

See:
- `test.c` - Original example (still works)
- `test_ergonomic.c` - New example demonstrating ergonomic APIs
