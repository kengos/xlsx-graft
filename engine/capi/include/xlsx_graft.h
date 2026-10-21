/*
 * xlsx-graft C ABI — the shared library the FFI SDKs load.
 *
 * Artifact: libxlsx_graft_capi.so / libxlsx_graft_capi.dylib /
 *           xlsx_graft_capi.dll
 *
 * This header is written by hand. `make engine:capi-abi` builds the library
 * and checks that the functions it exports are exactly the ones declared
 * here, and the crate's tests check that the status codes below have the
 * values the library returns.
 *
 * THE RULES THIS SURFACE IS BUILT FROM
 *
 * 1. Nothing is NUL-terminated. A string crosses as a pointer plus a length,
 *    and the bytes are UTF-8. Do not hand a returned pointer to a function
 *    that expects a C string.
 *
 * 2. What the library returns, the library owns. The version string is
 *    static: it lives as long as the library is loaded, and you never free
 *    it. Copy the bytes out if you need them past an unload.
 *
 * 3. A failure is a status, never an unwind. Every entry point returns an
 *    int32_t status, and a panic inside the library is caught at the
 *    boundary and comes back as XLSX_GRAFT_ERR_PANIC. Read the
 *    out-parameters only on XLSX_GRAFT_OK.
 *
 * WHAT IT CANNOT CHECK
 *
 *    That a non-null pointer you pass points at a writable slot of the right
 *    type. A null pointer is refused with XLSX_GRAFT_ERR_NULL_ARG; anything
 *    else that is not a valid slot is undefined behaviour, as it would be
 *    with any C function. Nor can you check a returned length: the library
 *    guarantees it, so read exactly that many bytes.
 *
 * THREADING
 *
 *    Every function may be called from several threads at once. The library
 *    holds no shared mutable state, so concurrent calls neither interfere nor
 *    need a lock of yours. A caught panic still runs Rust's process-wide
 *    panic hook, which by default prints a message to stderr.
 */

#ifndef XLSX_GRAFT_H
#define XLSX_GRAFT_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ---- status codes ---------------------------------------------------- */

/* The call completed and wrote its out-parameters. */
#define XLSX_GRAFT_OK 0
/* A pointer argument that must not be null was null. Nothing was written. */
#define XLSX_GRAFT_ERR_NULL_ARG 1
/* A panic was caught at the boundary. The library is still usable; the call
 * is not, and its out-parameters may have been partly written, so read none
 * of them. */
#define XLSX_GRAFT_ERR_PANIC 2

/* ---- functions ------------------------------------------------------- */

/*
 * The engine version, such as "0.0.0".
 *
 * On XLSX_GRAFT_OK, *out_ptr points at the version's UTF-8 bytes and
 * *out_len holds their count. The bytes are NOT NUL-terminated, are static,
 * and must not be freed. If either pointer is null the call returns
 * XLSX_GRAFT_ERR_NULL_ARG and writes nothing.
 */
int32_t xlsx_graft_version(const uint8_t **out_ptr, size_t *out_len);

#ifdef __cplusplus
}
#endif

#endif /* XLSX_GRAFT_H */
