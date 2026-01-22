#include <stdio.h>
#include <unistd.h>
#include "include/ninja.h"
#include "include/ninja_helpers.h"

/**
 * Example demonstrating the NEW ergonomic FFI API
 * 
 * This shows:
 * 1. Simplified error handling with ninja_has_error()
 * 2. Buffer-based error retrieval (no malloc needed)
 * 3. Simple function variants (ninja_start_shuriken vs ninja_start_shuriken_sync)
 * 4. Helper macros like NINJA_CHECK and NINJA_SCOPED_STRING
 * 5. Count API for quick queries
 */

typedef struct NinjaManagerOpaque NinjaManagerOpaque;
typedef void (*NinjaCallback)(void* userdata, const char* json);

void my_async_callback(void* userdata, const char* json) {
    printf("[async callback] userdata=%p json=%s\n", userdata, json);
    // IMPORTANT: free the string Rust gave us
    ninja_string_free((char*)json);
}

int main(void) {
    NinjaManagerOpaque* mgr = NULL;
    
    // Create manager - now with clearer error handling
    printf("Creating manager...\n");
    mgr = ninja_manager_new(NULL);
    NINJA_CHECK_NULL(mgr);
    printf("Manager created successfully\n");

    // Example 1: Get shuriken count (simple, no string allocation)
    printf("\n=== Example 1: Count shurikens ===\n");
    int count = ninja_count_shurikens(mgr);
    if (count >= 0) {
        printf("Found %d shurikens\n", count);
    } else {
        ninja_print_last_error("Failed to count shurikens");
    }

    // Example 2: List shurikens with NINJA_SCOPED_STRING (auto-cleanup on supported compilers)
    printf("\n=== Example 2: List shurikens ===\n");
    #if defined(__GNUC__) || defined(__clang__)
    {
        NINJA_SCOPED_STRING(list, ninja_list_shurikens_sync(mgr, NULL));
        if (list) {
            printf("Shurikens: %s\n", list);
            // list is automatically freed when leaving this scope
        } else {
            ninja_print_last_error("Failed to list shurikens");
        }
    }
    #else
    {
        char *list = ninja_list_shurikens_sync(mgr, NULL);
        if (list) {
            printf("Shurikens: %s\n", list);
            ninja_string_free(list);  // Manual cleanup on other compilers
        } else {
            ninja_print_last_error("Failed to list shurikens");
        }
    }
    #endif

    // Example 3: Use simple API with NINJA_CHECK macro
    printf("\n=== Example 3: Using NINJA_CHECK macro ===\n");
    NINJA_CHECK(ninja_start_shuriken(mgr, "apache"));
    printf("Started apache successfully\n");
    
    // Wait a bit
    usleep(500 * 1000);
    
    NINJA_CHECK(ninja_stop_shuriken(mgr, "apache"));
    printf("Stopped apache successfully\n");

    // Example 4: Manual error checking with buffer (no allocations)
    printf("\n=== Example 4: Buffer-based error checking ===\n");
    if (ninja_refresh_shuriken(mgr, "nonexistent") != 0) {
        char err_buf[256];
        int len = ninja_get_last_error_buf(err_buf, sizeof(err_buf));
        if (len > 0) {
            printf("Expected error: %s\n", err_buf);
        }
        ninja_clear_last_error();  // Clear for next operation
    }

    // Example 5: Async operations (unchanged, but still works)
    printf("\n=== Example 5: Async operations ===\n");
    ninja_start_shuriken_async(mgr, "apache", my_async_callback, (void*)0x1234);
    usleep(1000 * 1000);
    
    ninja_stop_shuriken_async(mgr, "apache", my_async_callback, (void*)0x4321);
    usleep(1000 * 1000);

    ninja_manager_free(mgr);
    printf("\nAll examples completed successfully!\n");
    return 0;

error:
    // Cleanup on error
    if (mgr) {
        ninja_manager_free(mgr);
    }
    return 1;
}
