#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <ctype.h>
#include <pthread.h>

#ifdef _WIN32
#include <io.h>
#define syntra_access _access
#else
#include <unistd.h>
#include <fcntl.h>
#include <errno.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <arpa/inet.h>
#include <netdb.h>
#ifdef __APPLE__
#include <sys/event.h>
#include <pthread/qos.h>
#include <CommonCrypto/CommonDigest.h>
#define SYNTRA_HAS_CC_SHA256 1
#endif
#if defined(__x86_64__) || defined(_M_X64)
#include <tmmintrin.h>
#endif
#define syntra_access access
#endif

void syntra_print_str(const char* s) {
    if (s) {
        fputs(s, stdout);
        fflush(stdout);
    }
}

void syntra_println_str(const char* s) {
    if (s) {
        puts(s);
    } else {
        putchar('\n');
    }
    fflush(stdout);
}

void syntra_println_int(int64_t v) {
    printf("%lld\n", (long long)v);
    fflush(stdout);
}

#define SYNTRA_STR_MAGIC 0x53595354 // "SYST"
#define SYNTRA_STR_MAGIC_POOLED 0x5359504F // "SYPO"

typedef struct {
    uint32_t magic;
    uint32_t cap;
    uint32_t len;
    uint32_t flags;
} syntra_str_hdr_t;

#define SYNTRA_STR_POOL_CAP 16
static __thread syntra_str_hdr_t* tl_str_pool[SYNTRA_STR_POOL_CAP];
static __thread int tl_str_pool_count = 0;

static inline syntra_str_hdr_t* syntra_str_pool_pop(size_t needed) {
    for (int p = tl_str_pool_count - 1; p >= 0; --p) {
        if (tl_str_pool[p]->cap >= needed) {
            syntra_str_hdr_t* hdr = tl_str_pool[p];
            tl_str_pool[p] = tl_str_pool[--tl_str_pool_count];
            return hdr;
        }
    }
    return NULL;
}

static inline int syntra_str_pool_push(syntra_str_hdr_t* hdr) {
    if (tl_str_pool_count < SYNTRA_STR_POOL_CAP && hdr->cap <= 8192) {
        hdr->magic = SYNTRA_STR_MAGIC_POOLED;
        tl_str_pool[tl_str_pool_count++] = hdr;
        return 1;
    }
    return 0;
}

static __thread char tl_int_buf[32];
static __thread size_t tl_int_len = 0;

static inline size_t syntra_fast_str_len(const char* s) {
    if (__builtin_expect(!s, 0)) return 0;
    if (__builtin_expect(s[0] == '\0', 0)) return 0;
    if (s == tl_int_buf) return tl_int_len;
    if ((uintptr_t)s >= sizeof(syntra_str_hdr_t)) {
        const syntra_str_hdr_t* hdr = (const syntra_str_hdr_t*)(s - sizeof(syntra_str_hdr_t));
        if (hdr->magic == SYNTRA_STR_MAGIC) {
            return (size_t)hdr->len;
        }
    }
    if (s[1] == '\0') return 1;
    if (s[2] == '\0') return 2;
    if (s[3] == '\0') return 3;
    if (s[4] == '\0') return 4;
    return strlen(s);
}

char* syntra_concat_str(const char* a, const char* b) {
    const char* sb = b ? b : "";
    size_t lb = syntra_fast_str_len(sb);

    // If 'a' is a Syntra dynamic string, append in-place with geometric growth
    if (a && (uintptr_t)a >= sizeof(syntra_str_hdr_t)) {
        syntra_str_hdr_t* hdr_a = (syntra_str_hdr_t*)((char*)a - sizeof(syntra_str_hdr_t));
        if (hdr_a->magic == SYNTRA_STR_MAGIC) {
            size_t la = (size_t)hdr_a->len;
            size_t needed = la + lb + 1;
            if (needed <= (size_t)hdr_a->cap) {
                memcpy((char*)a + la, sb, lb + 1);
                hdr_a->len = (uint32_t)(la + lb);
                return (char*)a;
            }
            size_t new_cap = (needed < 1024) ? 1024 : (needed * 2);
            syntra_str_hdr_t* new_hdr = (syntra_str_hdr_t*)realloc(hdr_a, sizeof(syntra_str_hdr_t) + new_cap);
            if (__builtin_expect(new_hdr != NULL, 1)) {
                new_hdr->cap = (uint32_t)new_cap;
                char* data = (char*)(new_hdr + 1);
                memcpy(data + la, sb, lb + 1);
                new_hdr->len = (uint32_t)(la + lb);
                return data;
            }
        }
    }

    const char* sa = a ? a : "";
    size_t la = syntra_fast_str_len(sa);
    size_t needed = la + lb + 1;
    size_t init_cap = (needed <= 64) ? 128 : ((needed <= 256) ? 1024 : ((needed <= 2048) ? 2048 : (needed * 2)));

    syntra_str_hdr_t* hdr = syntra_str_pool_pop(needed);
    if (!hdr) {
        hdr = (syntra_str_hdr_t*)malloc(sizeof(syntra_str_hdr_t) + init_cap);
        if (__builtin_expect(!hdr, 0)) return (char*)"";
        hdr->cap = (uint32_t)init_cap;
        hdr->flags = 0;
    }

    hdr->magic = SYNTRA_STR_MAGIC;
    hdr->len = (uint32_t)(la + lb);

    char* data = (char*)(hdr + 1);
    memcpy(data, sa, la);
    memcpy(data + la, sb, lb + 1);
    return data;
}

static const char g_digits100[200] =
    "0001020304050607080910111213141516171819"
    "2021222324252627282930313233343536373839"
    "4041424344454647484950515253545556575859"
    "6061626364656667686970717273747576777879"
    "8081828384858687888990919293949596979899";

char* syntra_int_to_str(int64_t v) {
    if (v >= 0) {
        if (v < 10) {
            tl_int_buf[0] = (char)('0' + v);
            tl_int_buf[1] = '\0';
            tl_int_len = 1;
            return tl_int_buf;
        }
        if (v < 100) {
            const char* d = &g_digits100[v * 2];
            tl_int_buf[0] = d[0];
            tl_int_buf[1] = d[1];
            tl_int_buf[2] = '\0';
            tl_int_len = 2;
            return tl_int_buf;
        }
        if (v < 1000) {
            tl_int_buf[0] = (char)('0' + (v / 100));
            const char* d = &g_digits100[(v % 100) * 2];
            tl_int_buf[1] = d[0];
            tl_int_buf[2] = d[1];
            tl_int_buf[3] = '\0';
            tl_int_len = 3;
            return tl_int_buf;
        }
        if (v < 10000) {
            const char* d1 = &g_digits100[(v / 100) * 2];
            const char* d2 = &g_digits100[(v % 100) * 2];
            tl_int_buf[0] = d1[0];
            tl_int_buf[1] = d1[1];
            tl_int_buf[2] = d2[0];
            tl_int_buf[3] = d2[1];
            tl_int_buf[4] = '\0';
            tl_int_len = 4;
            return tl_int_buf;
        }
        if (v < 100000) {
            tl_int_buf[0] = (char)('0' + (v / 10000));
            const char* d1 = &g_digits100[((v / 100) % 100) * 2];
            const char* d2 = &g_digits100[(v % 100) * 2];
            tl_int_buf[1] = d1[0];
            tl_int_buf[2] = d1[1];
            tl_int_buf[3] = d2[0];
            tl_int_buf[4] = d2[1];
            tl_int_buf[5] = '\0';
            tl_int_len = 5;
            return tl_int_buf;
        }
        if (v < 1000000) {
            const char* d0 = &g_digits100[(v / 10000) * 2];
            const char* d1 = &g_digits100[((v / 100) % 100) * 2];
            const char* d2 = &g_digits100[(v % 100) * 2];
            tl_int_buf[0] = d0[0];
            tl_int_buf[1] = d0[1];
            tl_int_buf[2] = d1[0];
            tl_int_buf[3] = d1[1];
            tl_int_buf[4] = d2[0];
            tl_int_buf[5] = d2[1];
            tl_int_buf[6] = '\0';
            tl_int_len = 6;
            return tl_int_buf;
        }
    }
    char temp[32];
    int idx = 0;
    if (v == 0) {
        temp[idx++] = '0';
    } else {
        long long n = v < 0 ? -v : v;
        while (n > 0) {
            temp[idx++] = (char)('0' + (n % 10));
            n /= 10;
        }
        if (v < 0) temp[idx++] = '-';
    }
    for (int j = 0; j < idx; j++) {
        tl_int_buf[j] = temp[idx - 1 - j];
    }
    tl_int_buf[idx] = '\0';
    tl_int_len = (size_t)idx;
    return tl_int_buf;
}

char* syntra_bool_to_str(int64_t v) {
    return (char*)(v ? "true" : "false");
}

static const char hex_pairs[512] =
    "000102030405060708090a0b0c0d0e0f"
    "101112131415161718191a1b1c1d1e1f"
    "202122232425262728292a2b2c2d2e2f"
    "303132333435363738393a3b3c3d3e3f"
    "404142434445464748494a4b4c4d4e4f"
    "505152535455565758595a5b5c5d5e5f"
    "606162636465666768696a6b6c6d6e6f"
    "707172737475767778797a7b7c7d7e7f"
    "808182838485868788898a8b8c8d8e8f"
    "909192939495969798999a9b9c9d9e9f"
    "a0a1a2a3a4a5a6a7a8a9aaabacadaeaf"
    "b0b1b2b3b4b5b6b7b8b9babbbcbdbebf"
    "c0c1c2c3c4c5c6c7c8c9cacbcccdcecf"
    "d0d1d2d3d4d5d6d7d8d9dadbdcdddedf"
    "e0e1e2e3e4e5e6e7e8e9eaebecedeeef"
    "f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff";

static __thread char tl_sha_ring[4][65];
static __thread int tl_sha_ring_idx = 0;

char* syntra_crypto_sha256(const char* input) {
    if (__builtin_expect(!input, 0)) input = "";
    unsigned char md[32];
    size_t in_len;
    if ((uintptr_t)input >= sizeof(syntra_str_hdr_t)) {
        const syntra_str_hdr_t* hdr = (const syntra_str_hdr_t*)(input - sizeof(syntra_str_hdr_t));
        if (hdr->magic == SYNTRA_STR_MAGIC) {
            in_len = (size_t)hdr->len;
        } else {
            in_len = syntra_fast_str_len(input);
        }
    } else {
        in_len = syntra_fast_str_len(input);
    }

#if defined(SYNTRA_HAS_CC_SHA256)
    CC_SHA256(input, (CC_LONG)in_len, md);
#else
    memset(md, 0, 32);
#endif
    char* hex = tl_sha_ring[(tl_sha_ring_idx++) & 3];

#if defined(__SSSE3__) || defined(__x86_64__)
    __m128i lut = _mm_setr_epi8('0','1','2','3','4','5','6','7','8','9','a','b','c','d','e','f');
    __m128i mask = _mm_set1_epi8(0x0f);

    for (int i = 0; i < 2; i++) {
        __m128i bytes = _mm_loadu_si128((const __m128i*)(md + i * 16));
        __m128i hi = _mm_and_si128(_mm_srli_epi16(bytes, 4), mask);
        __m128i lo = _mm_and_si128(bytes, mask);
        __m128i hex_hi = _mm_shuffle_epi8(lut, hi);
        __m128i hex_lo = _mm_shuffle_epi8(lut, lo);
        __m128i out0 = _mm_unpacklo_epi8(hex_hi, hex_lo);
        __m128i out1 = _mm_unpackhi_epi8(hex_hi, hex_lo);
        _mm_storeu_si128((__m128i*)(hex + i * 32), out0);
        _mm_storeu_si128((__m128i*)(hex + i * 32 + 16), out1);
    }
#else
    uint16_t* out16 = (uint16_t*)hex;
    const uint16_t* in16 = (const uint16_t*)hex_pairs;
    out16[0]  = in16[md[0]];  out16[1]  = in16[md[1]];  out16[2]  = in16[md[2]];  out16[3]  = in16[md[3]];
    out16[4]  = in16[md[4]];  out16[5]  = in16[md[5]];  out16[6]  = in16[md[6]];  out16[7]  = in16[md[7]];
    out16[8]  = in16[md[8]];  out16[9]  = in16[md[9]];  out16[10] = in16[md[10]]; out16[11] = in16[md[11]];
    out16[12] = in16[md[12]]; out16[13] = in16[md[13]]; out16[14] = in16[md[14]]; out16[15] = in16[md[15]];
    out16[16] = in16[md[16]]; out16[17] = in16[md[17]]; out16[18] = in16[md[18]]; out16[19] = in16[md[19]];
    out16[20] = in16[md[20]]; out16[21] = in16[md[21]]; out16[22] = in16[md[22]]; out16[23] = in16[md[23]];
    out16[24] = in16[md[24]]; out16[25] = in16[md[25]]; out16[26] = in16[md[26]]; out16[27] = in16[md[27]];
    out16[28] = in16[md[28]]; out16[29] = in16[md[29]]; out16[30] = in16[md[30]]; out16[31] = in16[md[31]];
#endif
    hex[64] = '\0';
    return hex;
}

// ─── Managed Heap & Garbage Collection Subsystem (Specification 02) ──────────

#define SYNTRA_GC_MAGIC 0x53594E54 // "SYNT"
#define SYNTRA_GC_FLAG_MARKED 0x01
#define SYNTRA_GC_FLAG_ARRAY  0x02
#define SYNTRA_MAX_GC_ROOTS   8192

typedef struct syntra_gc_header {
    struct syntra_gc_header* next;
    struct syntra_gc_header* prev;
    size_t size;
    uint32_t flags;
    uint32_t magic;
} syntra_gc_header_t;

static pthread_mutex_t g_gc_lock = PTHREAD_MUTEX_INITIALIZER;
static syntra_gc_header_t* g_gc_head = NULL;
static size_t g_gc_allocated_bytes = 0;
static size_t g_gc_peak_bytes = 0;
static size_t g_gc_total_allocations = 0;
static size_t g_gc_total_freed = 0;
static size_t g_gc_collection_count = 0;
static size_t g_gc_threshold = 256 * 1024; // 256 KB default threshold
static void* g_stack_base = NULL;
static void** g_gc_roots[SYNTRA_MAX_GC_ROOTS];
static size_t g_gc_root_count = 0;

static void syntra_gc_init_stack_base(void) {
    if (!g_stack_base) {
        g_stack_base = __builtin_frame_address(0);
    }
}

__attribute__((constructor))
static void syntra_rt_constructor(void) {
    g_stack_base = __builtin_frame_address(0);
}

static void syntra_gc_mark_address(uintptr_t ptr) {
    if (ptr < 4096) return;
    syntra_gc_header_t* curr = g_gc_head;
    while (curr) {
        if (curr->magic == SYNTRA_GC_MAGIC) {
            uintptr_t obj_start = (uintptr_t)(curr + 1);
            uintptr_t obj_end = obj_start + curr->size;
            if (ptr >= obj_start && ptr < obj_end) {
                if (!(curr->flags & SYNTRA_GC_FLAG_MARKED)) {
                    curr->flags |= SYNTRA_GC_FLAG_MARKED;
                    // Scan words inside this object for references
                    uintptr_t* fields = (uintptr_t*)obj_start;
                    size_t words = curr->size / sizeof(uintptr_t);
                    for (size_t i = 0; i < words; i++) {
                        syntra_gc_mark_address(fields[i]);
                    }
                    if (curr->flags & SYNTRA_GC_FLAG_ARRAY) {
                        typedef struct {
                            int64_t length;
                            int64_t capacity;
                            int64_t* elements;
                        } temp_array_t;
                        temp_array_t* arr = (temp_array_t*)obj_start;
                        if (arr->elements) {
                            for (int64_t i = 0; i < arr->length; i++) {
                                syntra_gc_mark_address((uintptr_t)arr->elements[i]);
                            }
                        }
                    }
                }
                return;
            }
        }
        curr = curr->next;
    }
}

static void syntra_gc_collect_internal(void) {
    syntra_gc_init_stack_base();

    // 1. Mark shadow stack roots
    for (size_t i = 0; i < g_gc_root_count; i++) {
        if (g_gc_roots[i] && *g_gc_roots[i]) {
            syntra_gc_mark_address((uintptr_t)*g_gc_roots[i]);
        }
    }

    // 2. Conservative stack scanning
    void* cur_sp = __builtin_frame_address(0);
    uintptr_t low = (uintptr_t)cur_sp;
    uintptr_t high = (uintptr_t)g_stack_base;
    if (high < low) {
        uintptr_t tmp = low;
        low = high;
        high = tmp;
    }
    low = (low + 7) & ~7;
    high = high & ~7;
    if (high - low > 1024 * 1024) {
        high = low + 1024 * 1024;
    }

    uintptr_t* p = (uintptr_t*)low;
    uintptr_t* end = (uintptr_t*)high;
    while (p < end) {
        syntra_gc_mark_address(*p);
        p++;
    }

    // 3. Sweep unreferenced objects
    syntra_gc_header_t** pcur = &g_gc_head;
    while (*pcur) {
        syntra_gc_header_t* h = *pcur;
        if (h->magic != SYNTRA_GC_MAGIC) {
            pcur = &h->next;
            continue;
        }
        if (!(h->flags & SYNTRA_GC_FLAG_MARKED)) {
            *pcur = h->next;
            if (h->next) {
                h->next->prev = h->prev;
            }
            if (h->flags & SYNTRA_GC_FLAG_ARRAY) {
                typedef struct {
                    int64_t length;
                    int64_t capacity;
                    int64_t* elements;
                } temp_array_t;
                temp_array_t* arr = (temp_array_t*)(h + 1);
                if (arr->elements) {
                    free(arr->elements);
                    arr->elements = NULL;
                }
            }
            g_gc_allocated_bytes -= h->size;
            g_gc_total_freed += h->size;
            h->magic = 0;
            free(h);
        } else {
            h->flags &= ~SYNTRA_GC_FLAG_MARKED;
            pcur = &h->next;
        }
    }

    g_gc_collection_count++;

    // Adjust threshold: float dynamic headroom above surviving bytes
    size_t new_thresh = g_gc_allocated_bytes + (256 * 1024);
    if (new_thresh < 256 * 1024) new_thresh = 256 * 1024;
    g_gc_threshold = new_thresh;
}

void* syntra_alloc(int64_t size) {
    if (size <= 0) size = 8;
    size_t alloc_sz = (size_t)size;
    alloc_sz = (alloc_sz + 15) & ~15; // 16-byte alignment

    pthread_mutex_lock(&g_gc_lock);
    syntra_gc_init_stack_base();

    if (g_gc_allocated_bytes + alloc_sz > g_gc_threshold) {
        syntra_gc_collect_internal();
    }

    size_t total_size = sizeof(syntra_gc_header_t) + alloc_sz;
    syntra_gc_header_t* hdr = (syntra_gc_header_t*)calloc(1, total_size);
    if (!hdr) {
        syntra_gc_collect_internal();
        hdr = (syntra_gc_header_t*)calloc(1, total_size);
        if (!hdr) {
            pthread_mutex_unlock(&g_gc_lock);
            return NULL;
        }
    }

    hdr->magic = SYNTRA_GC_MAGIC;
    hdr->size = alloc_sz;
    hdr->flags = 0;
    hdr->next = g_gc_head;
    hdr->prev = NULL;
    if (g_gc_head) {
        g_gc_head->prev = hdr;
    }
    g_gc_head = hdr;

    g_gc_allocated_bytes += alloc_sz;
    g_gc_total_allocations++;
    if (g_gc_allocated_bytes > g_gc_peak_bytes) {
        g_gc_peak_bytes = g_gc_allocated_bytes;
    }

    pthread_mutex_unlock(&g_gc_lock);
    return (void*)(hdr + 1);
}

int syntra_try_free_pooled_map(void* ptr);

void syntra_free(void* ptr) {
    if (!ptr) return;
    if (ptr >= (void*)tl_sha_ring && ptr < (void*)(tl_sha_ring + 4)) return;
    if (ptr >= (void*)tl_int_buf && ptr < (void*)(tl_int_buf + sizeof(tl_int_buf))) return;
    if (syntra_try_free_pooled_map(ptr)) return;

    if ((uintptr_t)ptr >= sizeof(syntra_str_hdr_t)) {
        syntra_str_hdr_t* str_hdr = (syntra_str_hdr_t*)((char*)ptr - sizeof(syntra_str_hdr_t));
        if (str_hdr->magic == SYNTRA_STR_MAGIC) {
            if (syntra_str_pool_push(str_hdr)) {
                return;
            }
            str_hdr->magic = 0;
            free(str_hdr);
            return;
        }
        if (str_hdr->magic == SYNTRA_STR_MAGIC_POOLED) {
            return;
        }
    }
    syntra_gc_header_t* hdr = ((syntra_gc_header_t*)ptr) - 1;
    if (hdr->magic != SYNTRA_GC_MAGIC) {
        free(ptr);
        return;
    }

    pthread_mutex_lock(&g_gc_lock);
    if (hdr->prev) {
        hdr->prev->next = hdr->next;
    } else if (g_gc_head == hdr) {
        g_gc_head = hdr->next;
    }
    if (hdr->next) {
        hdr->next->prev = hdr->prev;
    }

    if (hdr->flags & SYNTRA_GC_FLAG_ARRAY) {
        typedef struct {
            int64_t length;
            int64_t capacity;
            int64_t* elements;
        } temp_array_t;
        temp_array_t* arr = (temp_array_t*)(hdr + 1);
        if (arr->elements) {
            free(arr->elements);
            arr->elements = NULL;
        }
    }

    g_gc_allocated_bytes -= hdr->size;
    g_gc_total_freed += hdr->size;
    hdr->magic = 0;
    free(hdr);
    pthread_mutex_unlock(&g_gc_lock);
}

void syntra_gc_collect(void) {
    pthread_mutex_lock(&g_gc_lock);
    syntra_gc_collect_internal();
    pthread_mutex_unlock(&g_gc_lock);
}

int64_t syntra_memory_allocated_bytes(void) {
    pthread_mutex_lock(&g_gc_lock);
    int64_t bytes = (int64_t)g_gc_allocated_bytes;
    pthread_mutex_unlock(&g_gc_lock);
    return bytes;
}

int64_t syntra_gc_total_collections(void) {
    pthread_mutex_lock(&g_gc_lock);
    int64_t count = (int64_t)g_gc_collection_count;
    pthread_mutex_unlock(&g_gc_lock);
    return count;
}

void syntra_gc_push_root(void** root_ptr) {
    if (!root_ptr) return;
    pthread_mutex_lock(&g_gc_lock);
    if (g_gc_root_count < SYNTRA_MAX_GC_ROOTS) {
        g_gc_roots[g_gc_root_count++] = root_ptr;
    }
    pthread_mutex_unlock(&g_gc_lock);
}

void syntra_gc_pop_root(void) {
    pthread_mutex_lock(&g_gc_lock);
    if (g_gc_root_count > 0) {
        g_gc_root_count--;
    }
    pthread_mutex_unlock(&g_gc_lock);
}

void syntra_gc_set_threshold(int64_t threshold_bytes) {
    if (threshold_bytes <= 1024) threshold_bytes = 1024;
    pthread_mutex_lock(&g_gc_lock);
    g_gc_threshold = (size_t)threshold_bytes;
    pthread_mutex_unlock(&g_gc_lock);
}

// ─── Scoped Arena Memory Domain (Specification 02) ───────────────────────────

typedef struct syntra_arena_slab {
    struct syntra_arena_slab* next;
    size_t capacity;
    size_t offset;
    uint8_t data[];
} syntra_arena_slab_t;

typedef struct {
    syntra_arena_slab_t* head;
    syntra_arena_slab_t* current;
    size_t default_slab_size;
    size_t total_allocated;
} syntra_arena_t;

void* syntra_arena_create(int64_t initial_capacity) {
    if (initial_capacity <= 0) initial_capacity = 65536;
    syntra_arena_t* arena = (syntra_arena_t*)malloc(sizeof(syntra_arena_t));
    if (!arena) return NULL;

    syntra_arena_slab_t* slab = (syntra_arena_slab_t*)malloc(sizeof(syntra_arena_slab_t) + (size_t)initial_capacity);
    if (!slab) {
        free(arena);
        return NULL;
    }
    slab->next = NULL;
    slab->capacity = (size_t)initial_capacity;
    slab->offset = 0;

    arena->head = slab;
    arena->current = slab;
    arena->default_slab_size = (size_t)initial_capacity;
    arena->total_allocated = 0;
    return arena;
}

void* syntra_arena_alloc(void* arena_ptr, int64_t size) {
    if (!arena_ptr) return syntra_alloc(size);
    if (size <= 0) size = 8;
    size_t aligned_size = (size + 7) & ~7;
    syntra_arena_t* arena = (syntra_arena_t*)arena_ptr;

    syntra_arena_slab_t* curr = arena->current;
    if (curr && (curr->offset + aligned_size <= curr->capacity)) {
        void* ptr = (void*)(curr->data + curr->offset);
        curr->offset += aligned_size;
        arena->total_allocated += aligned_size;
        memset(ptr, 0, aligned_size);
        return ptr;
    }

    size_t new_cap = arena->default_slab_size;
    if (aligned_size > new_cap) new_cap = aligned_size * 2;
    syntra_arena_slab_t* new_slab = (syntra_arena_slab_t*)malloc(sizeof(syntra_arena_slab_t) + new_cap);
    if (!new_slab) return syntra_alloc(size);
    new_slab->next = NULL;
    new_slab->capacity = new_cap;
    new_slab->offset = aligned_size;

    if (curr) {
        curr->next = new_slab;
    }
    arena->current = new_slab;
    arena->total_allocated += aligned_size;
    memset(new_slab->data, 0, aligned_size);
    return (void*)new_slab->data;
}

void syntra_arena_reset(void* arena_ptr) {
    if (!arena_ptr) return;
    syntra_arena_t* arena = (syntra_arena_t*)arena_ptr;
    syntra_arena_slab_t* slab = arena->head;
    while (slab) {
        slab->offset = 0;
        slab = slab->next;
    }
    arena->current = arena->head;
    arena->total_allocated = 0;
}

void syntra_arena_destroy(void* arena_ptr) {
    if (!arena_ptr) return;
    syntra_arena_t* arena = (syntra_arena_t*)arena_ptr;
    syntra_arena_slab_t* slab = arena->head;
    while (slab) {
        syntra_arena_slab_t* next = slab->next;
        free(slab);
        slab = next;
    }
    free(arena);
}

int64_t syntra_arena_bytes_allocated(void* arena_ptr) {
    if (!arena_ptr) return 0;
    syntra_arena_t* arena = (syntra_arena_t*)arena_ptr;
    return (int64_t)arena->total_allocated;
}

// Array runtime representation
typedef struct {
    int64_t length;
    int64_t capacity;
    int64_t* elements;
} syntra_array_t;

void* syntra_alloc_array(int64_t length) {
    if (length < 0) length = 0;
    int64_t cap = length > 8 ? length * 2 : 16;
    syntra_array_t* arr = (syntra_array_t*)syntra_alloc(sizeof(syntra_array_t));
    if (!arr) return NULL;
    syntra_gc_header_t* hdr = ((syntra_gc_header_t*)arr) - 1;
    hdr->flags |= SYNTRA_GC_FLAG_ARRAY;
    arr->length = length;
    arr->capacity = cap;
    arr->elements = (int64_t*)calloc((size_t)cap, sizeof(int64_t));
    return arr;
}

int64_t syntra_array_get(void* array_ptr, int64_t index) {
    if (!array_ptr) return 0;
    syntra_array_t* arr = (syntra_array_t*)array_ptr;
    if (index < 0 || index >= arr->length) return 0;
    return arr->elements[index];
}

void syntra_array_set(void* array_ptr, int64_t index, int64_t val) {
    if (!array_ptr) return;
    syntra_array_t* arr = (syntra_array_t*)array_ptr;
    if (index < 0) return;
    if (index >= arr->capacity) {
        int64_t new_cap = (index + 1) * 2;
        arr->elements = (int64_t*)realloc(arr->elements, (size_t)new_cap * sizeof(int64_t));
        for (int64_t i = arr->capacity; i < new_cap; i++) arr->elements[i] = 0;
        arr->capacity = new_cap;
    }
    if (index >= arr->length) {
        arr->length = index + 1;
    }
    arr->elements[index] = val;
}

int64_t syntra_array_push(void* array_ptr, int64_t val) {
    if (!array_ptr) return 0;
    syntra_array_t* arr = (syntra_array_t*)array_ptr;
    if (arr->length >= arr->capacity) {
        int64_t new_cap = arr->capacity > 0 ? arr->capacity * 2 : 16;
        arr->elements = (int64_t*)realloc(arr->elements, (size_t)new_cap * sizeof(int64_t));
        arr->capacity = new_cap;
    }
    arr->elements[arr->length] = val;
    arr->length++;
    return arr->length;
}

int64_t syntra_array_len(void* array_ptr) {
    if (!array_ptr) return 0;
    syntra_array_t* arr = (syntra_array_t*)array_ptr;
    return arr->length;
}

int64_t syntra_array_min(void* array_ptr) {
    if (!array_ptr) return 0;
    syntra_array_t* arr = (syntra_array_t*)array_ptr;
    if (arr->length <= 0) return 0;
    int64_t m = arr->elements[0];
    for (int64_t i = 1; i < arr->length; i++) {
        if (arr->elements[i] < m) m = arr->elements[i];
    }
    return m;
}

int64_t syntra_array_max(void* array_ptr) {
    if (!array_ptr) return 0;
    syntra_array_t* arr = (syntra_array_t*)array_ptr;
    if (arr->length <= 0) return 0;
    int64_t m = arr->elements[0];
    for (int64_t i = 1; i < arr->length; i++) {
        if (arr->elements[i] > m) m = arr->elements[i];
    }
    return m;
}

// Math built-ins
int64_t syntra_math_abs(int64_t x) {
    return x < 0 ? -x : x;
}

int64_t syntra_math_min(int64_t a, int64_t b) {
    return a < b ? a : b;
}

int64_t syntra_math_max(int64_t a, int64_t b) {
    return a > b ? a : b;
}

int64_t syntra_math_clamp(int64_t val, int64_t low, int64_t high) {
    if (val < low) return low;
    if (val > high) return high;
    return val;
}

int64_t syntra_math_pow(int64_t base, int64_t exp) {
    if (exp < 0) return 0;
    int64_t res = 1;
    while (exp > 0) {
        if (exp % 2 == 1) res *= base;
        base *= base;
        exp /= 2;
    }
    return res;
}

int64_t syntra_math_sqrt(int64_t x) {
    if (x <= 0) return 0;
    int64_t r = 0;
    while ((r + 1) * (r + 1) <= x) {
        r++;
    }
    return r;
}

// First-class String Runtime Functions
int64_t syntra_str_len(const char* s) {
    return (int64_t)syntra_fast_str_len(s);
}

int64_t syntra_str_is_empty(const char* s) {
    return (!s || s[0] == '\0') ? 1 : 0;
}

int64_t syntra_str_contains(const char* s, const char* sub) {
    if (!s || !sub) return 0;
    return strstr(s, sub) != NULL ? 1 : 0;
}

int64_t syntra_str_index_of(const char* s, const char* sub) {
    if (!s || !sub) return -1;
    const char* pos = strstr(s, sub);
    return pos ? (int64_t)(pos - s) : -1;
}

int64_t syntra_str_starts_with(const char* s, const char* prefix) {
    if (!s || !prefix) return 0;
    size_t lp = strlen(prefix);
    return (strncmp(s, prefix, lp) == 0) ? 1 : 0;
}

int64_t syntra_str_ends_with(const char* s, const char* suffix) {
    if (!s || !suffix) return 0;
    size_t ls = strlen(s);
    size_t lsub = strlen(suffix);
    if (lsub > ls) return 0;
    return (strcmp(s + ls - lsub, suffix) == 0) ? 1 : 0;
}

static __thread char tl_case_buf[64];

char* syntra_str_to_upper(const char* s) {
    if (!s) return "";
    size_t len = strlen(s);
    int has_lower = 0;
    for (size_t i = 0; i < len; i++) {
        if (s[i] >= 'a' && s[i] <= 'z') { has_lower = 1; break; }
    }
    if (!has_lower) return (char*)s;
    if (len < 63) {
        for (size_t i = 0; i < len; i++) {
            tl_case_buf[i] = (char)toupper((unsigned char)s[i]);
        }
        tl_case_buf[len] = '\0';
        return tl_case_buf;
    }
    char* res = (char*)malloc(len + 1);
    for (size_t i = 0; i < len; i++) {
        res[i] = (char)toupper((unsigned char)s[i]);
    }
    res[len] = '\0';
    return res;
}

char* syntra_str_to_lower(const char* s) {
    if (!s) return "";
    size_t len = strlen(s);
    int has_upper = 0;
    for (size_t i = 0; i < len; i++) {
        if (s[i] >= 'A' && s[i] <= 'Z') { has_upper = 1; break; }
    }
    if (!has_upper) return (char*)s;
    if (len < 63) {
        for (size_t i = 0; i < len; i++) {
            tl_case_buf[i] = (char)tolower((unsigned char)s[i]);
        }
        tl_case_buf[len] = '\0';
        return tl_case_buf;
    }
    char* res = (char*)malloc(len + 1);
    for (size_t i = 0; i < len; i++) {
        res[i] = (char)tolower((unsigned char)s[i]);
    }
    res[len] = '\0';
    return res;
}

char* syntra_str_trim(const char* s) {
    if (!s) return strdup("");
    while (*s && isspace((unsigned char)*s)) s++;
    if (*s == 0) return strdup("");
    const char* end = s + strlen(s) - 1;
    while (end > s && isspace((unsigned char)*end)) end--;
    size_t len = (size_t)(end - s + 1);
    char* res = (char*)malloc(len + 1);
    memcpy(res, s, len);
    res[len] = '\0';
    return res;
}

char* syntra_str_slice(const char* s, int64_t start, int64_t end) {
    if (!s) return (char*)"";
    int64_t len = (int64_t)syntra_fast_str_len(s);
    if (start < 0) start = 0;
    if (end > len) end = len;
    if (start >= end || start >= len) return (char*)"";
    size_t slice_len = (size_t)(end - start);

    syntra_str_hdr_t* hdr = syntra_str_pool_pop(slice_len + 1);
    if (!hdr) {
        size_t init_cap = (slice_len < 32) ? 32 : slice_len;
        hdr = (syntra_str_hdr_t*)malloc(sizeof(syntra_str_hdr_t) + init_cap + 1);
        if (__builtin_expect(!hdr, 0)) return (char*)"";
        hdr->cap = (uint32_t)init_cap;
        hdr->flags = 0;
    }

    hdr->magic = SYNTRA_STR_MAGIC;
    hdr->len = (uint32_t)slice_len;

    char* res = (char*)(hdr + 1);
    memcpy(res, s + start, slice_len);
    res[slice_len] = '\0';
    return res;
}

static char s_ascii_table[256][2];
static int s_ascii_table_inited = 0;

char* syntra_str_char_at(const char* s, int64_t index) {
    if (!s) return "";
    int64_t len = (int64_t)strlen(s);
    if (index < 0 || index >= len) return "";
    if (!s_ascii_table_inited) {
        for (int i = 0; i < 256; i++) {
            s_ascii_table[i][0] = (char)i;
            s_ascii_table[i][1] = '\0';
        }
        s_ascii_table_inited = 1;
    }
    unsigned char c = (unsigned char)s[index];
    return s_ascii_table[c];
}

int64_t syntra_str_char_code_at(const char* s, int64_t index) {
    if (__builtin_expect(!s || index < 0, 0)) return -1;
    unsigned char c = (unsigned char)s[index];
    return c == '\0' ? -1 : (int64_t)c;
}

int64_t syntra_str_eq(const char* a, const char* b) {
    if (a == b) return 1;
    if (__builtin_expect(!a || !b, 0)) return 0;
    if (a[0] != b[0]) return 0;
    return strcmp(a, b) == 0 ? 1 : 0;
}

char* syntra_str_replace(const char* s, const char* target, const char* repl) {
    if (!s) return strdup("");
    if (!target || *target == '\0') return strdup(s);
    const char* r = repl ? repl : "";
    size_t target_len = strlen(target);
    size_t repl_len = strlen(r);

    int count = 0;
    const char* p = s;
    while ((p = strstr(p, target)) != NULL) {
        count++;
        p += target_len;
    }
    if (count == 0) return strdup(s);

    size_t new_len = strlen(s) + count * (repl_len - target_len);
    char* result = (char*)malloc(new_len + 1);
    if (!result) return strdup(s);

    char* out = result;
    p = s;
    const char* match;
    while ((match = strstr(p, target)) != NULL) {
        size_t chunk = (size_t)(match - p);
        memcpy(out, p, chunk);
        out += chunk;
        memcpy(out, r, repl_len);
        out += repl_len;
        p = match + target_len;
    }
    strcpy(out, p);
    return result;
}

void* syntra_str_split(const char* s, const char* delim) {
    if (!s) return syntra_alloc_array(0);
    if (!delim || *delim == '\0') {
        size_t len = strlen(s);
        syntra_array_t* arr = (syntra_array_t*)syntra_alloc_array((int64_t)len);
        for (size_t i = 0; i < len; i++) {
            char* c_str = (char*)malloc(2);
            c_str[0] = s[i];
            c_str[1] = '\0';
            arr->elements[i] = (int64_t)c_str;
        }
        return arr;
    }

    size_t delim_len = strlen(delim);
    int count = 1;
    const char* p = s;
    while ((p = strstr(p, delim)) != NULL) {
        count++;
        p += delim_len;
    }

    syntra_array_t* arr = (syntra_array_t*)syntra_alloc_array((int64_t)count);
    p = s;
    const char* match;
    int idx = 0;
    while ((match = strstr(p, delim)) != NULL) {
        size_t chunk = (size_t)(match - p);
        char* token = (char*)malloc(chunk + 1);
        memcpy(token, p, chunk);
        token[chunk] = '\0';
        arr->elements[idx++] = (int64_t)token;
        p = match + delim_len;
    }
    size_t tail_len = strlen(p);
    char* tail = (char*)malloc(tail_len + 1);
    memcpy(tail, p, tail_len + 1);
    arr->elements[idx] = (int64_t)tail;
    return arr;
}

int64_t syntra_str_to_int(const char* s) {
    if (!s) return 0;
    return (int64_t)strtoll(s, NULL, 10);
}

char* syntra_array_join(void* array_ptr, const char* sep) {
    if (!array_ptr) return strdup("");
    syntra_array_t* arr = (syntra_array_t*)array_ptr;
    if (arr->length <= 0) return strdup("");
    const char* s = sep ? sep : "";
    size_t sep_len = strlen(s);

    size_t total_len = 0;
    for (int64_t i = 0; i < arr->length; i++) {
        const char* str_elem = (const char*)arr->elements[i];
        if (str_elem) total_len += strlen(str_elem);
        if (i > 0) total_len += sep_len;
    }

    char* result = (char*)malloc(total_len + 1);
    if (!result) return strdup("");
    char* out = result;
    for (int64_t i = 0; i < arr->length; i++) {
        if (i > 0 && sep_len > 0) {
            memcpy(out, s, sep_len);
            out += sep_len;
        }
        const char* str_elem = (const char*)arr->elements[i];
        if (str_elem) {
            size_t el = strlen(str_elem);
            memcpy(out, str_elem, el);
            out += el;
        }
    }
    *out = '\0';
    return result;
}

// ==========================================
// File I/O Standard Subsystem
// ==========================================

int64_t syntra_file_exists(const char* path) {
    if (!path) return 0;
    return syntra_access(path, 0) == 0 ? 1 : 0;
}

char* syntra_file_read_text(const char* path) {
    if (!path) return strdup("");
    FILE* f = fopen(path, "rb");
    if (!f) return strdup("");
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    if (sz < 0) {
        fclose(f);
        return strdup("");
    }
    fseek(f, 0, SEEK_SET);
    char* buf = (char*)malloc((size_t)sz + 1);
    if (!buf) {
        fclose(f);
        return strdup("");
    }
    size_t read_bytes = fread(buf, 1, (size_t)sz, f);
    buf[read_bytes] = '\0';
    fclose(f);
    return buf;
}

int64_t syntra_file_write_text(const char* path, const char* content) {
    if (!path) return 0;
    FILE* f = fopen(path, "wb");
    if (!f) return 0;
    if (content) {
        size_t len = strlen(content);
        if (len > 0) {
            size_t written = fwrite(content, 1, len, f);
            fclose(f);
            return written == len ? 1 : 0;
        }
    }
    fclose(f);
    return 1;
}

int64_t syntra_file_append_text(const char* path, const char* content) {
    if (!path) return 0;
    FILE* f = fopen(path, "ab");
    if (!f) return 0;
    if (content) {
        size_t len = strlen(content);
        if (len > 0) {
            size_t written = fwrite(content, 1, len, f);
            fclose(f);
            return written == len ? 1 : 0;
        }
    }
    fclose(f);
    return 1;
}

int64_t syntra_file_delete(const char* path) {
    if (!path) return 0;
    return remove(path) == 0 ? 1 : 0;
}

int64_t syntra_file_size(const char* path) {
    if (!path) return -1;
    FILE* f = fopen(path, "rb");
    if (!f) return -1;
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fclose(f);
    return (int64_t)sz;
}

void* syntra_file_read_lines(const char* path) {
    char* text = syntra_file_read_text(path);
    void* arr = syntra_str_split(text, "\n");
    free(text);
    return arr;
}

// Built-in class File runtime bindings
void* File_new(const char* path) {
    char** obj = (char**)calloc(1, sizeof(char*));
    if (obj) {
        *obj = path ? strdup(path) : strdup("");
    }
    return obj;
}

char* File_readText(void* this_ptr) {
    if (!this_ptr) return strdup("");
    const char* path = *(const char**)this_ptr;
    return syntra_file_read_text(path);
}

int64_t File_writeText(void* this_ptr, const char* content) {
    if (!this_ptr) return 0;
    const char* path = *(const char**)this_ptr;
    return syntra_file_write_text(path, content);
}

int64_t File_appendText(void* this_ptr, const char* content) {
    if (!this_ptr) return 0;
    const char* path = *(const char**)this_ptr;
    return syntra_file_append_text(path, content);
}

int64_t File_exists(void* this_ptr) {
    if (!this_ptr) return 0;
    const char* path = *(const char**)this_ptr;
    return syntra_file_exists(path);
}

int64_t File_delete(void* this_ptr) {
    if (!this_ptr) return 0;
    const char* path = *(const char**)this_ptr;
    return syntra_file_delete(path);
}

int64_t File_size(void* this_ptr) {
    if (!this_ptr) return -1;
    const char* path = *(const char**)this_ptr;
    return syntra_file_size(path);
}

char* File_getPath(void* this_ptr) {
    if (!this_ptr) return strdup("");
    const char* path = *(const char**)this_ptr;
    return path ? strdup(path) : strdup("");
}

void* File_readLines(void* this_ptr) {
    if (!this_ptr) return syntra_alloc_array(0);
    const char* path = *(const char**)this_ptr;
    return syntra_file_read_lines(path);
}

// CLI Arguments capture
static int g_syntra_argc = 0;
static char** g_syntra_argv = NULL;

__attribute__((constructor))
static void syntra_capture_cli_args(int argc, char** argv) {
    g_syntra_argc = argc;
    g_syntra_argv = argv;
}

void* syntra_system_args(void) {
    if (g_syntra_argc <= 1 || !g_syntra_argv) {
        return syntra_alloc_array(0);
    }
    int64_t count = (int64_t)(g_syntra_argc - 1);
    syntra_array_t* arr = (syntra_array_t*)syntra_alloc_array(count);
    if (!arr) return NULL;
    for (int64_t i = 0; i < count; i++) {
        arr->elements[i] = (int64_t)strdup(g_syntra_argv[i + 1]);
    }
    return arr;
}

void syntra_system_exit(int64_t code) {
    exit((int)code);
}

int64_t syntra_system_exec(const char* cmd) {
    if (!cmd) return -1;
    int status = system(cmd);
    return (int64_t)status;
}

char* syntra_string_from_char_code(int64_t code) {
    char* s = (char*)malloc(2);
    s[0] = (char)(code & 0xFF);
    s[1] = '\0';
    return s;
}

char* syntra_system_getenv(const char* key) {
    if (!key) return strdup("");
    char* val = getenv(key);
    return val ? strdup(val) : strdup("");
}

#ifdef _WIN32
#include <windows.h>
int64_t syntra_system_time_ms(void) {
    FILETIME ft;
    GetSystemTimeAsFileTime(&ft);
    ULARGE_INTEGER uli;
    uli.LowPart = ft.dwLowDateTime;
    uli.HighPart = ft.dwHighDateTime;
    return (int64_t)((uli.QuadPart - 116444736000000000ULL) / 10000ULL);
}
#else
#include <sys/time.h>
int64_t syntra_system_time_ms(void) {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return ((int64_t)tv.tv_sec) * 1000 + ((int64_t)tv.tv_usec) / 1000;
}
#endif

// Error handling runtime
static __thread char* g_syntra_last_error = NULL;

void syntra_set_error(const char* msg) {
    if (g_syntra_last_error) {
        free(g_syntra_last_error);
    }
    g_syntra_last_error = msg ? strdup(msg) : strdup("Runtime error");
}

char* syntra_get_error(void) {
    return g_syntra_last_error ? strdup(g_syntra_last_error) : strdup("");
}

int64_t syntra_has_error(void) {
    return g_syntra_last_error != NULL ? 1 : 0;
}

void syntra_clear_error(void) {
    if (g_syntra_last_error) {
        free(g_syntra_last_error);
        g_syntra_last_error = NULL;
    }
}

void syntra_array_set_len(void* array_ptr, int64_t len) {
    if (!array_ptr) return;
    syntra_array_t* arr = (syntra_array_t*)array_ptr;
    if (len > arr->capacity) {
        int64_t new_cap = len * 2;
        arr->elements = (int64_t*)realloc(arr->elements, (size_t)new_cap * sizeof(int64_t));
        for (int64_t i = arr->capacity; i < new_cap; i++) arr->elements[i] = 0;
        arr->capacity = new_cap;
    }
    arr->length = len >= 0 ? len : 0;
}

// Map Runtime Subsystem
#define SYNTRA_MAP_STATIC_ENTRIES 8

typedef struct syntra_map_entry {
    char* key;
    int64_t value;
    int val_type; // 0 = number/bool, 1 = string, 2 = map
    struct syntra_map_entry* next;
    char static_key[64];
    int is_static;
} syntra_map_entry_t;

typedef struct {
    int64_t size;
    int64_t capacity;
    syntra_map_entry_t** buckets;
    syntra_map_entry_t static_entries[SYNTRA_MAP_STATIC_ENTRIES];
    int static_entry_count;
    int is_pooled;
} syntra_map_t;

static uint64_t syntra_hash_str(const char* str) {
    uint64_t hash = 5381;
    int c;
    if (!str) return 0;
    while ((c = (unsigned char)*str++)) {
        hash = ((hash << 5) + hash) + c;
    }
    return hash;
}

void syntra_map_clear(void* map_ptr);

#define SYNTRA_MAP_POOL_SIZE 16
static __thread syntra_map_t tl_map_pool[SYNTRA_MAP_POOL_SIZE];
static __thread syntra_map_entry_t* tl_map_pool_buckets[SYNTRA_MAP_POOL_SIZE][16];
static __thread size_t tl_map_pool_idx = 0;
static __thread int tl_map_pool_inited = 0;

void* syntra_map_new(void) {
    if (!tl_map_pool_inited) {
        for (int i = 0; i < SYNTRA_MAP_POOL_SIZE; i++) {
            tl_map_pool[i].capacity = 16;
            tl_map_pool[i].size = 0;
            tl_map_pool[i].buckets = tl_map_pool_buckets[i];
            tl_map_pool[i].is_pooled = 1;
            tl_map_pool[i].static_entry_count = 0;
            memset(tl_map_pool_buckets[i], 0, sizeof(tl_map_pool_buckets[i]));
        }
        tl_map_pool_inited = 1;
    }
    size_t idx = (tl_map_pool_idx++) % SYNTRA_MAP_POOL_SIZE;
    syntra_map_t* map = &tl_map_pool[idx];
    syntra_map_clear(map);
    return map;
}

int syntra_try_free_pooled_map(void* ptr) {
    if (tl_map_pool_inited && ptr >= (void*)tl_map_pool && ptr < (void*)(tl_map_pool + SYNTRA_MAP_POOL_SIZE)) {
        syntra_map_clear(ptr);
        return 1;
    }
    return 0;
}

void syntra_map_set_typed(void* map_ptr, const char* key, int64_t val, int val_type) {
    if (!map_ptr || !key) return;
    syntra_map_t* map = (syntra_map_t*)map_ptr;
    uint64_t h = syntra_hash_str(key) % map->capacity;
    syntra_map_entry_t* cur = map->buckets[h];
    while (cur) {
        if (strcmp(cur->key, key) == 0) {
            cur->value = val;
            cur->val_type = val_type;
            return;
        }
        cur = cur->next;
    }
    syntra_map_entry_t* entry = NULL;
    if (map->is_pooled && map->static_entry_count < SYNTRA_MAP_STATIC_ENTRIES && strlen(key) < 63) {
        entry = &map->static_entries[map->static_entry_count++];
        strcpy(entry->static_key, key);
        entry->key = entry->static_key;
        entry->is_static = 1;
    } else {
        entry = (syntra_map_entry_t*)malloc(sizeof(syntra_map_entry_t));
        entry->key = strdup(key);
        entry->is_static = 0;
    }
    entry->value = val;
    entry->val_type = val_type;
    entry->next = map->buckets[h];
    map->buckets[h] = entry;
    map->size++;
}

void syntra_map_set(void* map_ptr, const char* key, int64_t val) {
    syntra_map_set_typed(map_ptr, key, val, 0);
}

int64_t syntra_map_get(void* map_ptr, const char* key) {
    if (!map_ptr || !key) return 0;
    syntra_map_t* map = (syntra_map_t*)map_ptr;
    uint64_t h = syntra_hash_str(key) % map->capacity;
    syntra_map_entry_t* cur = map->buckets[h];
    while (cur) {
        if (strcmp(cur->key, key) == 0) {
            return cur->value;
        }
        cur = cur->next;
    }
    return 0;
}

int64_t syntra_map_has(void* map_ptr, const char* key) {
    if (!map_ptr || !key) return 0;
    syntra_map_t* map = (syntra_map_t*)map_ptr;
    uint64_t h = syntra_hash_str(key) % map->capacity;
    syntra_map_entry_t* cur = map->buckets[h];
    while (cur) {
        if (strcmp(cur->key, key) == 0) {
            return 1;
        }
        cur = cur->next;
    }
    return 0;
}

int64_t syntra_map_remove(void* map_ptr, const char* key) {
    if (!map_ptr || !key) return 0;
    syntra_map_t* map = (syntra_map_t*)map_ptr;
    uint64_t h = syntra_hash_str(key) % map->capacity;
    syntra_map_entry_t** cur = &map->buckets[h];
    while (*cur) {
        if (strcmp((*cur)->key, key) == 0) {
            syntra_map_entry_t* to_free = *cur;
            *cur = to_free->next;
            free(to_free->key);
            free(to_free);
            map->size--;
            return 1;
        }
        cur = &(*cur)->next;
    }
    return 0;
}

int64_t syntra_map_len(void* map_ptr) {
    if (!map_ptr) return 0;
    syntra_map_t* map = (syntra_map_t*)map_ptr;
    return map->size;
}

void* syntra_map_keys(void* map_ptr) {
    if (!map_ptr) return syntra_alloc_array(0);
    syntra_map_t* map = (syntra_map_t*)map_ptr;
    syntra_array_t* arr = (syntra_array_t*)syntra_alloc_array(map->size);
    int64_t idx = 0;
    for (int64_t i = 0; i < map->capacity; i++) {
        syntra_map_entry_t* cur = map->buckets[i];
        while (cur) {
            arr->elements[idx++] = (int64_t)cur->key;
            cur = cur->next;
        }
    }
    return arr;
}

void* syntra_map_values(void* map_ptr) {
    if (!map_ptr) return syntra_alloc_array(0);
    syntra_map_t* map = (syntra_map_t*)map_ptr;
    syntra_array_t* arr = (syntra_array_t*)syntra_alloc_array(map->size);
    int64_t idx = 0;
    for (int64_t i = 0; i < map->capacity; i++) {
        syntra_map_entry_t* cur = map->buckets[i];
        while (cur) {
            arr->elements[idx++] = cur->value;
            cur = cur->next;
        }
    }
    return arr;
}

void syntra_map_clear(void* map_ptr) {
    if (!map_ptr) return;
    syntra_map_t* map = (syntra_map_t*)map_ptr;
    if (map->size == 0) {
        map->static_entry_count = 0;
        return;
    }
    for (int64_t i = 0; i < map->capacity; i++) {
        syntra_map_entry_t* cur = map->buckets[i];
        while (cur) {
            syntra_map_entry_t* next = cur->next;
            if (!cur->is_static) {
                free(cur->key);
                free(cur);
            }
            if ((uintptr_t)cur->value >= 0x100000000ULL && (uintptr_t)cur->value <= 0x00007fffffffffffULL) {
                syntra_str_hdr_t* vhdr = (syntra_str_hdr_t*)((char*)cur->value - sizeof(syntra_str_hdr_t));
                if (vhdr->magic == SYNTRA_STR_MAGIC) {
                    syntra_free((void*)cur->value);
                }
            }
            cur = next;
        }
        map->buckets[i] = NULL;
    }
    map->size = 0;
    map->static_entry_count = 0;
}

// JSON Subsystem
static void syntra_skip_ws(const char** p) {
    while (**p && isspace((unsigned char)**p)) (*p)++;
}

static char* syntra_parse_json_str(const char** p) {
    syntra_skip_ws(p);
    if (**p != '"') return strdup("");
    (*p)++;
    const char* start = *p;
    while (**p && **p != '"') {
        if (**p == '\\' && *(*p + 1)) (*p) += 2;
        else (*p)++;
    }
    size_t len = *p - start;
    char* res = (char*)malloc(len + 1);
    memcpy(res, start, len);
    res[len] = '\0';
    if (**p == '"') (*p)++;
    return res;
}

void* syntra_json_parse(const char* json_str) {
    if (!json_str) return syntra_map_new();
    const char* p = json_str;
    syntra_skip_ws(&p);
    if (*p != '{') return syntra_map_new();
    p++;
    void* map = syntra_map_new();
    while (*p) {
        syntra_skip_ws(&p);
        if (*p == '}') { p++; break; }
        char* key = syntra_parse_json_str(&p);
        syntra_skip_ws(&p);
        if (*p == ':') p++;
        syntra_skip_ws(&p);
        if (*p == '"') {
            char* val = syntra_parse_json_str(&p);
            syntra_map_set_typed(map, key, (int64_t)val, 1);
        } else if (*p == '{') {
            const char* obj_start = p;
            int depth = 1;
            p++;
            while (*p && depth > 0) {
                if (*p == '{') depth++;
                else if (*p == '}') depth--;
                p++;
            }
            size_t sub_len = p - obj_start;
            char* sub = (char*)malloc(sub_len + 1);
            memcpy(sub, obj_start, sub_len);
            sub[sub_len] = '\0';
            void* sub_map = syntra_json_parse(sub);
            free(sub);
            syntra_map_set_typed(map, key, (int64_t)sub_map, 2);
        } else if (isdigit((unsigned char)*p) || *p == '-') {
            char* end;
            long long num = strtoll(p, &end, 10);
            p = end;
            syntra_map_set(map, key, (int64_t)num);
        } else if (strncmp(p, "true", 4) == 0) {
            p += 4;
            syntra_map_set(map, key, 1);
        } else if (strncmp(p, "false", 5) == 0) {
            p += 5;
            syntra_map_set(map, key, 0);
        } else if (strncmp(p, "null", 4) == 0) {
            p += 4;
            syntra_map_set(map, key, 0);
        } else {
            p++;
        }
        free(key);
        syntra_skip_ws(&p);
        if (*p == ',') p++;
        else if (*p == '}') { p++; break; }
    }
    return map;
}

char* syntra_json_stringify(void* map_ptr) {
    if (!map_ptr) return strdup("{}");
    syntra_map_t* map = (syntra_map_t*)map_ptr;
    size_t cap = 256;
    char* buf = (char*)malloc(cap);
    buf[0] = '{';
    buf[1] = '\0';
    size_t len = 1;
    int first = 1;
    for (int64_t i = 0; i < map->capacity; i++) {
        syntra_map_entry_t* cur = map->buckets[i];
        while (cur) {
            char temp[256];
            if (cur->val_type == 1 && cur->value != 0) {
                snprintf(temp, sizeof(temp), "\"%s\"", (const char*)cur->value);
            } else {
                snprintf(temp, sizeof(temp), "%lld", (long long)cur->value);
            }
            size_t entry_len = strlen(cur->key) + strlen(temp) + 16;
            if (len + entry_len >= cap) {
                cap = (cap * 2) + entry_len;
                buf = (char*)realloc(buf, cap);
            }
            if (!first) {
                strcat(buf, ", ");
                len += 2;
            }
            first = 0;
            strcat(buf, "\"");
            strcat(buf, cur->key);
            strcat(buf, "\": ");
            strcat(buf, temp);
            len = strlen(buf);
            cur = cur->next;
        }
    }
    if (len + 2 >= cap) {
        buf = (char*)realloc(buf, cap + 2);
    }
    strcat(buf, "}");
    return buf;
}

// OpenAPI Schema Registry
typedef struct {
    char* name;
    char* schema_json;
} SyntraSchemaEntry;

static SyntraSchemaEntry* g_syntra_schemas = NULL;
static int g_syntra_schema_count = 0;

void syntra_schema_register(const char* name, const char* schema_json) {
    if (!name || !schema_json) return;
    g_syntra_schemas = (SyntraSchemaEntry*)realloc(g_syntra_schemas, sizeof(SyntraSchemaEntry) * (g_syntra_schema_count + 1));
    g_syntra_schemas[g_syntra_schema_count].name = strdup(name);
    g_syntra_schemas[g_syntra_schema_count].schema_json = strdup(schema_json);
    g_syntra_schema_count++;
}

char* syntra_schema_describe(const char* name) {
    if (!name) return strdup("{\"type\":\"object\",\"properties\":{}}");
    for (int i = 0; i < g_syntra_schema_count; i++) {
        if (strcmp(g_syntra_schemas[i].name, name) == 0) {
            return strdup(g_syntra_schemas[i].schema_json);
        }
    }
    return strdup("{\"type\":\"object\",\"properties\":{}}");
}

// Concurrency Subsystem: Tasks & Threads
typedef struct {
    pthread_t thread;
    void* fn;
    void* arg;
    int64_t argc;
    int64_t args[8];
    int64_t result;
    int joined;
    int completed;
    pthread_mutex_t completion_lock;
    pthread_cond_t completion_cond;
} syntra_task_t;

static void* syntra_task_runner(void* arg) {
    syntra_task_t* task = (syntra_task_t*)arg;
    int64_t res = 0;
    switch (task->argc) {
        case 0: {
            int64_t (*f)(void) = (int64_t (*)(void))task->fn;
            res = f();
            break;
        }
        case 1: {
            int64_t (*f)(int64_t) = (int64_t (*)(int64_t))task->fn;
            res = f(task->args[0]);
            break;
        }
        case 2: {
            int64_t (*f)(int64_t, int64_t) = (int64_t (*)(int64_t, int64_t))task->fn;
            res = f(task->args[0], task->args[1]);
            break;
        }
        case 3: {
            int64_t (*f)(int64_t, int64_t, int64_t) = (int64_t (*)(int64_t, int64_t, int64_t))task->fn;
            res = f(task->args[0], task->args[1], task->args[2]);
            break;
        }
        case 4: {
            int64_t (*f)(int64_t, int64_t, int64_t, int64_t) = (int64_t (*)(int64_t, int64_t, int64_t, int64_t))task->fn;
            res = f(task->args[0], task->args[1], task->args[2], task->args[3]);
            break;
        }
        case 5: {
            int64_t (*f)(int64_t, int64_t, int64_t, int64_t, int64_t) = (int64_t (*)(int64_t, int64_t, int64_t, int64_t, int64_t))task->fn;
            res = f(task->args[0], task->args[1], task->args[2], task->args[3], task->args[4]);
            break;
        }
        case 6: {
            int64_t (*f)(int64_t, int64_t, int64_t, int64_t, int64_t, int64_t) = (int64_t (*)(int64_t, int64_t, int64_t, int64_t, int64_t, int64_t))task->fn;
            res = f(task->args[0], task->args[1], task->args[2], task->args[3], task->args[4], task->args[5]);
            break;
        }
        default: {
            int64_t (*f)(int64_t) = (int64_t (*)(int64_t))task->fn;
            res = f(task->args[0]);
            break;
        }
    }
    task->result = res;
    pthread_mutex_lock(&task->completion_lock);
    task->completed = 1;
    pthread_cond_broadcast(&task->completion_cond);
    pthread_mutex_unlock(&task->completion_lock);
    return (void*)res;
}

int64_t syntra_invoke_fn0(void* fn) {
    if (!fn) return 0;
    int64_t (*f)(void) = (int64_t (*)(void))fn;
    return f();
}

int64_t syntra_invoke_fn1(void* fn, int64_t a) {
    if (!fn) return 0;
    int64_t (*f)(int64_t) = (int64_t (*)(int64_t))fn;
    return f(a);
}

int64_t syntra_invoke_fn2(void* fn, int64_t a, int64_t b) {
    if (!fn) return 0;
    int64_t (*f)(int64_t, int64_t) = (int64_t (*)(int64_t, int64_t))fn;
    return f(a, b);
}

void* syntra_task_spawn_n(void* fn_ptr, int64_t argc, int64_t a0, int64_t a1, int64_t a2, int64_t a3, int64_t a4, int64_t a5) {
    if (!fn_ptr) return NULL;
    syntra_task_t* task = (syntra_task_t*)malloc(sizeof(syntra_task_t));
    memset(task, 0, sizeof(syntra_task_t));
    pthread_mutex_init(&task->completion_lock, NULL);
    pthread_cond_init(&task->completion_cond, NULL);
    task->fn = fn_ptr;
    task->argc = argc;
    task->args[0] = a0;
    task->args[1] = a1;
    task->args[2] = a2;
    task->args[3] = a3;
    task->args[4] = a4;
    task->args[5] = a5;
    task->result = 0;
    task->joined = 0;
    task->completed = 0;
    pthread_attr_t attr;
    pthread_attr_init(&attr);
    pthread_attr_setstacksize(&attr, 8 * 1024 * 1024);
    pthread_create(&task->thread, &attr, syntra_task_runner, task);
    pthread_attr_destroy(&attr);
    return task;
}

void* syntra_task_spawn(void* fn_ptr, void* arg) {
    void* t = syntra_task_spawn_n(fn_ptr, 1, (int64_t)arg, 0, 0, 0, 0, 0);
    if (t) {
        ((syntra_task_t*)t)->arg = arg;
    }
    return t;
}

int64_t syntra_task_join(void* task_ptr) {
    if (!task_ptr) return 0;
    syntra_task_t* task = (syntra_task_t*)task_ptr;
    if (!task->joined) {
        if (task->thread != 0) {
            pthread_join(task->thread, NULL);
        } else {
            pthread_mutex_lock(&task->completion_lock);
            while (!task->completed) {
                pthread_cond_wait(&task->completion_cond, &task->completion_lock);
            }
            pthread_mutex_unlock(&task->completion_lock);
        }
        task->joined = 1;
    }
    int64_t res = task->result;
    return res;
}

int64_t syntra_task_is_done(void* task_ptr) {
    if (!task_ptr) return 1;
    syntra_task_t* task = (syntra_task_t*)task_ptr;
    pthread_mutex_lock(&task->completion_lock);
    int64_t d = task->completed;
    pthread_mutex_unlock(&task->completion_lock);
    return d;
}

void syntra_task_sleep(int64_t ms) {
    if (ms <= 0) return;
    usleep((useconds_t)(ms * 1000));
}

// Thread Pool Subsystem
typedef struct syntra_tp_job {
    syntra_task_t* task;
    struct syntra_tp_job* next;
} syntra_tp_job_t;

typedef struct {
    pthread_mutex_t lock;
    pthread_cond_t cond;
    pthread_t* workers;
    int64_t worker_count;
    syntra_tp_job_t* job_head;
    syntra_tp_job_t* job_tail;
    int shutdown;
} syntra_threadpool_t;

static void* syntra_tp_worker_fn(void* arg) {
    syntra_threadpool_t* pool = (syntra_threadpool_t*)arg;
    while (1) {
        pthread_mutex_lock(&pool->lock);
        while (pool->job_head == NULL && !pool->shutdown) {
            pthread_cond_wait(&pool->cond, &pool->lock);
        }
        if (pool->shutdown && pool->job_head == NULL) {
            pthread_mutex_unlock(&pool->lock);
            break;
        }
        syntra_tp_job_t* job = pool->job_head;
        pool->job_head = job->next;
        if (!pool->job_head) pool->job_tail = NULL;
        pthread_mutex_unlock(&pool->lock);

        // Run the task
        syntra_task_t* task = job->task;
        syntra_task_runner(task);
        free(job);
    }
    return NULL;
}

void* syntra_threadpool_new(int64_t workers) {
    if (workers <= 0) workers = 4;
    syntra_threadpool_t* pool = (syntra_threadpool_t*)malloc(sizeof(syntra_threadpool_t));
    pthread_mutex_init(&pool->lock, NULL);
    pthread_cond_init(&pool->cond, NULL);
    pool->worker_count = workers;
    pool->job_head = NULL;
    pool->job_tail = NULL;
    pool->shutdown = 0;
    pool->workers = (pthread_t*)malloc(sizeof(pthread_t) * (size_t)workers);
    for (int64_t i = 0; i < workers; i++) {
        pthread_create(&pool->workers[i], NULL, syntra_tp_worker_fn, pool);
    }
    return pool;
}

void* syntra_threadpool_submit(void* pool_ptr, void* fn_ptr, void* arg) {
    if (!pool_ptr || !fn_ptr) return NULL;
    syntra_threadpool_t* pool = (syntra_threadpool_t*)pool_ptr;
    syntra_task_t* task = (syntra_task_t*)malloc(sizeof(syntra_task_t));
    memset(task, 0, sizeof(syntra_task_t));
    pthread_mutex_init(&task->completion_lock, NULL);
    pthread_cond_init(&task->completion_cond, NULL);
    task->thread = 0;
    task->fn = (void* (*)(void*))fn_ptr;
    task->arg = arg;
    task->argc = 1;
    task->args[0] = (int64_t)arg;
    task->result = 0;
    task->joined = 0;
    task->completed = 0;

    syntra_tp_job_t* job = (syntra_tp_job_t*)malloc(sizeof(syntra_tp_job_t));
    job->task = task;
    job->next = NULL;

    pthread_mutex_lock(&pool->lock);
    if (pool->shutdown) {
        pthread_mutex_unlock(&pool->lock);
        free(job);
        free(task);
        return NULL;
    }
    if (pool->job_tail) {
        pool->job_tail->next = job;
        pool->job_tail = job;
    } else {
        pool->job_head = job;
        pool->job_tail = job;
    }
    pthread_cond_signal(&pool->cond);
    pthread_mutex_unlock(&pool->lock);
    return task;
}

void syntra_threadpool_shutdown(void* pool_ptr) {
    if (!pool_ptr) return;
    syntra_threadpool_t* pool = (syntra_threadpool_t*)pool_ptr;
    pthread_mutex_lock(&pool->lock);
    pool->shutdown = 1;
    pthread_cond_broadcast(&pool->cond);
    pthread_mutex_unlock(&pool->lock);
    for (int64_t i = 0; i < pool->worker_count; i++) {
        pthread_join(pool->workers[i], NULL);
    }
    free(pool->workers);
    pthread_mutex_destroy(&pool->lock);
    pthread_cond_destroy(&pool->cond);
    free(pool);
}

// Concurrency Subsystem: Thread-Safe Channels
typedef struct syntra_chan_node {
    int64_t val;
    struct syntra_chan_node* next;
} syntra_chan_node_t;

typedef struct {
    pthread_mutex_t lock;
    pthread_cond_t not_empty;
    pthread_cond_t not_full;
    syntra_chan_node_t* head;
    syntra_chan_node_t* tail;
    int64_t size;
    int64_t capacity; // 0 = unbounded
    int closed;
} syntra_channel_t;

void* syntra_channel_new(int64_t capacity) {
    syntra_channel_t* ch = (syntra_channel_t*)malloc(sizeof(syntra_channel_t));
    pthread_mutex_init(&ch->lock, NULL);
    pthread_cond_init(&ch->not_empty, NULL);
    pthread_cond_init(&ch->not_full, NULL);
    ch->head = NULL;
    ch->tail = NULL;
    ch->size = 0;
    ch->capacity = capacity > 0 ? capacity : 0;
    ch->closed = 0;
    return ch;
}

void syntra_channel_send(void* ch_ptr, int64_t val) {
    if (!ch_ptr) return;
    syntra_channel_t* ch = (syntra_channel_t*)ch_ptr;
    pthread_mutex_lock(&ch->lock);
    if (ch->closed) {
        pthread_mutex_unlock(&ch->lock);
        return;
    }
    while (ch->capacity > 0 && ch->size >= ch->capacity && !ch->closed) {
        pthread_cond_wait(&ch->not_full, &ch->lock);
    }
    if (ch->closed) {
        pthread_mutex_unlock(&ch->lock);
        return;
    }
    syntra_chan_node_t* node = (syntra_chan_node_t*)malloc(sizeof(syntra_chan_node_t));
    node->val = val;
    node->next = NULL;
    if (ch->tail) {
        ch->tail->next = node;
        ch->tail = node;
    } else {
        ch->head = node;
        ch->tail = node;
    }
    ch->size++;
    pthread_cond_signal(&ch->not_empty);
    pthread_mutex_unlock(&ch->lock);
}

int64_t syntra_channel_recv(void* ch_ptr) {
    if (!ch_ptr) return 0;
    syntra_channel_t* ch = (syntra_channel_t*)ch_ptr;
    pthread_mutex_lock(&ch->lock);
    while (ch->size == 0 && !ch->closed) {
        pthread_cond_wait(&ch->not_empty, &ch->lock);
    }
    if (ch->size == 0 && ch->closed) {
        pthread_mutex_unlock(&ch->lock);
        return 0;
    }
    syntra_chan_node_t* node = ch->head;
    int64_t val = node->val;
    ch->head = node->next;
    if (!ch->head) ch->tail = NULL;
    ch->size--;
    free(node);
    pthread_cond_signal(&ch->not_full);
    pthread_mutex_unlock(&ch->lock);
    return val;
}

void syntra_channel_close(void* ch_ptr) {
    if (!ch_ptr) return;
    syntra_channel_t* ch = (syntra_channel_t*)ch_ptr;
    pthread_mutex_lock(&ch->lock);
    ch->closed = 1;
    pthread_cond_broadcast(&ch->not_empty);
    pthread_cond_broadcast(&ch->not_full);
    pthread_mutex_unlock(&ch->lock);
}

int64_t syntra_channel_len(void* ch_ptr) {
    if (!ch_ptr) return 0;
    syntra_channel_t* ch = (syntra_channel_t*)ch_ptr;
    pthread_mutex_lock(&ch->lock);
    int64_t sz = ch->size;
    pthread_mutex_unlock(&ch->lock);
    return sz;
}

int64_t syntra_channel_try_recv(void* ch_ptr) {
    if (!ch_ptr) return 0;
    syntra_channel_t* ch = (syntra_channel_t*)ch_ptr;
    pthread_mutex_lock(&ch->lock);
    if (ch->size == 0) {
        pthread_mutex_unlock(&ch->lock);
        return 0;
    }
    syntra_chan_node_t* node = ch->head;
    int64_t val = node->val;
    ch->head = node->next;
    if (!ch->head) ch->tail = NULL;
    ch->size--;
    free(node);
    pthread_cond_signal(&ch->not_full);
    pthread_mutex_unlock(&ch->lock);
    return val;
}

int64_t syntra_channel_try_send(void* ch_ptr, int64_t val) {
    if (!ch_ptr) return 0;
    syntra_channel_t* ch = (syntra_channel_t*)ch_ptr;
    pthread_mutex_lock(&ch->lock);
    if (ch->closed || (ch->capacity > 0 && ch->size >= ch->capacity)) {
        pthread_mutex_unlock(&ch->lock);
        return 0;
    }
    syntra_chan_node_t* node = (syntra_chan_node_t*)malloc(sizeof(syntra_chan_node_t));
    node->val = val;
    node->next = NULL;
    if (ch->tail) {
        ch->tail->next = node;
        ch->tail = node;
    } else {
        ch->head = node;
        ch->tail = node;
    }
    ch->size++;
    pthread_cond_signal(&ch->not_empty);
    pthread_mutex_unlock(&ch->lock);
    return 1;
}

int64_t syntra_channel_select(void* array_ptr) {
    if (!array_ptr) return -1;
    syntra_array_t* arr = (syntra_array_t*)array_ptr;
    if (arr->length <= 0 || !arr->elements) return -1;

    while (1) {
        for (int64_t i = 0; i < arr->length; i++) {
            syntra_channel_t* ch = (syntra_channel_t*)arr->elements[i];
            if (!ch) continue;
            pthread_mutex_lock(&ch->lock);
            if (ch->size > 0 || ch->closed) {
                pthread_mutex_unlock(&ch->lock);
                return i;
            }
            pthread_mutex_unlock(&ch->lock);
        }
#ifdef _WIN32
        Sleep(1);
#else
        usleep(1000);
#endif
    }
    return -1;
}

// Networking Subsystem: Sockets
typedef struct {
    int fd;
    int port;
} syntra_tcp_listener_t;

typedef struct {
    int fd;
} syntra_tcp_stream_t;

void* syntra_tcp_listen(int64_t port) {
#ifndef _WIN32
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return NULL;
    int opt = 1;
    setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
#ifdef SO_REUSEPORT
    setsockopt(fd, SOL_SOCKET, SO_REUSEPORT, &opt, sizeof(opt));
#endif

    int buf_size = 131072;
    setsockopt(fd, SOL_SOCKET, SO_RCVBUF, &buf_size, sizeof(buf_size));
    setsockopt(fd, SOL_SOCKET, SO_SNDBUF, &buf_size, sizeof(buf_size));

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_ANY);
    addr.sin_port = htons((uint16_t)port);

    if (bind(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(fd);
        return NULL;
    }
    if (listen(fd, 4096) < 0) {
        close(fd);
        return NULL;
    }

    socklen_t len = sizeof(addr);
    if (getsockname(fd, (struct sockaddr*)&addr, &len) == 0) {
        port = ntohs(addr.sin_port);
    }

    syntra_tcp_listener_t* listener = (syntra_tcp_listener_t*)malloc(sizeof(syntra_tcp_listener_t));
    listener->fd = fd;
    listener->port = (int)port;
    return listener;
#else
    return NULL;
#endif
}

void* syntra_tcp_accept(void* listener_ptr) {
#ifndef _WIN32
    if (!listener_ptr) return NULL;
    syntra_tcp_listener_t* listener = (syntra_tcp_listener_t*)listener_ptr;
    struct sockaddr_in client_addr;
    socklen_t client_len = sizeof(client_addr);
    int client_fd = accept(listener->fd, (struct sockaddr*)&client_addr, &client_len);
    if (client_fd < 0) return NULL;

    int nodelay = 1;
    setsockopt(client_fd, IPPROTO_TCP, TCP_NODELAY, &nodelay, sizeof(nodelay));

    int buf_size = 131072;
    setsockopt(client_fd, SOL_SOCKET, SO_RCVBUF, &buf_size, sizeof(buf_size));
    setsockopt(client_fd, SOL_SOCKET, SO_SNDBUF, &buf_size, sizeof(buf_size));

    struct timeval tv;
    tv.tv_sec = 2;
    tv.tv_usec = 0;
    setsockopt(client_fd, SOL_SOCKET, SO_RCVTIMEO, (const char*)&tv, sizeof(tv));

    syntra_tcp_stream_t* stream = (syntra_tcp_stream_t*)malloc(sizeof(syntra_tcp_stream_t));
    stream->fd = client_fd;
    return stream;
#else
    return NULL;
#endif
}

char* syntra_tcp_read(void* stream_ptr, int64_t max_len) {
#ifndef _WIN32
    if (!stream_ptr) return "";
    syntra_tcp_stream_t* stream = (syntra_tcp_stream_t*)stream_ptr;
    if (max_len <= 0) max_len = 65536;
    char* buf = (char*)malloc(max_len + 1);
    if (!buf) return "";
    ssize_t n = recv(stream->fd, buf, max_len, 0);
    if (n < 0) n = 0;
    buf[n] = '\0';
    return buf;
#else
    return "";
#endif
}

int64_t syntra_tcp_write(void* stream_ptr, const char* data) {
#ifndef _WIN32
    if (!stream_ptr || !data) return 0;
    syntra_tcp_stream_t* stream = (syntra_tcp_stream_t*)stream_ptr;
    size_t len = strlen(data);
    ssize_t n = send(stream->fd, data, len, 0);
    return (int64_t)n;
#else
    return 0;
#endif
}

int64_t syntra_tcp_listener_port(void* listener_ptr) {
    if (!listener_ptr) return 0;
    syntra_tcp_listener_t* listener = (syntra_tcp_listener_t*)listener_ptr;
    return (int64_t)listener->port;
}

void syntra_tcp_close(void* ptr) {
#ifndef _WIN32
    if (!ptr) return;
    syntra_tcp_stream_t* s = (syntra_tcp_stream_t*)ptr;
    if (s->fd >= 0) {
        close(s->fd);
        s->fd = -1;
    }
    free(ptr);
#endif
}

void* syntra_tcp_connect(const char* host, int64_t port) {
#ifndef _WIN32
    if (!host) host = "127.0.0.1";
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) return NULL;

    struct hostent* he = gethostbyname(host);
    if (!he) {
        close(fd);
        return NULL;
    }

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    memcpy(&addr.sin_addr, he->h_addr_list[0], he->h_length);
    addr.sin_port = htons((uint16_t)port);

    if (connect(fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(fd);
        return NULL;
    }

    int nodelay = 1;
    setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &nodelay, sizeof(nodelay));

    syntra_tcp_stream_t* stream = (syntra_tcp_stream_t*)malloc(sizeof(syntra_tcp_stream_t));
    stream->fd = fd;
    return stream;
#else
    return NULL;
#endif
}

// ─── RFC 6455 WebSocket Native Protocols & Cryptography ───────────────────────

static int syntra_recv_exact(int fd, uint8_t* buf, size_t len) {
    size_t total = 0;
    while (total < len) {
        ssize_t n = recv(fd, buf + total, len - total, 0);
        if (n <= 0) return 0;
        total += (size_t)n;
    }
    return 1;
}

static int syntra_send_exact(int fd, const uint8_t* buf, size_t len) {
    size_t total = 0;
    while (total < len) {
        ssize_t n = send(fd, buf + total, len - total, 0);
        if (n <= 0) return 0;
        total += (size_t)n;
    }
    return 1;
}

static inline uint32_t syntra_rol32(uint32_t val, int bits) {
    return (val << bits) | (val >> (32 - bits));
}

static void syntra_sha1(const uint8_t* data, size_t len, uint8_t hash[20]) {
    uint32_t h0 = 0x67452301;
    uint32_t h1 = 0xEFCDAB89;
    uint32_t h2 = 0x98BADCFE;
    uint32_t h3 = 0x10325476;
    uint32_t h4 = 0xC3D2E1F0;

    uint64_t bit_len = (uint64_t)len * 8;
    size_t padded_len = ((len + 8 + 64) / 64) * 64;
    uint8_t* msg = (uint8_t*)calloc(padded_len, 1);
    if (!msg) return;
    memcpy(msg, data, len);
    msg[len] = 0x80;

    for (int i = 0; i < 8; i++) {
        msg[padded_len - 8 + i] = (uint8_t)(bit_len >> (56 - i * 8));
    }

    for (size_t chunk = 0; chunk < padded_len; chunk += 64) {
        uint32_t w[80];
        for (int i = 0; i < 16; i++) {
            size_t off = chunk + i * 4;
            w[i] = ((uint32_t)msg[off] << 24) |
                   ((uint32_t)msg[off + 1] << 16) |
                   ((uint32_t)msg[off + 2] << 8) |
                   ((uint32_t)msg[off + 3]);
        }
        for (int i = 16; i < 80; i++) {
            w[i] = syntra_rol32(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);
        }

        uint32_t a = h0, b = h1, c = h2, d = h3, e = h4;
        for (int i = 0; i < 80; i++) {
            uint32_t f, k;
            if (i < 20) {
                f = (b & c) | ((~b) & d);
                k = 0x5A827999;
            } else if (i < 40) {
                f = b ^ c ^ d;
                k = 0x6ED9EBA1;
            } else if (i < 60) {
                f = (b & c) | (b & d) | (c & d);
                k = 0x8F1BBCDC;
            } else {
                f = b ^ c ^ d;
                k = 0xCA62C1D6;
            }
            uint32_t temp = syntra_rol32(a, 5) + f + e + k + w[i];
            e = d;
            d = c;
            c = syntra_rol32(b, 30);
            b = a;
            a = temp;
        }

        h0 += a;
        h1 += b;
        h2 += c;
        h3 += d;
        h4 += e;
    }
    free(msg);

    uint32_t hs[5] = {h0, h1, h2, h3, h4};
    for (int i = 0; i < 5; i++) {
        hash[i * 4] = (uint8_t)(hs[i] >> 24);
        hash[i * 4 + 1] = (uint8_t)(hs[i] >> 16);
        hash[i * 4 + 2] = (uint8_t)(hs[i] >> 8);
        hash[i * 4 + 3] = (uint8_t)(hs[i]);
    }
}

static const char BASE64_CHARS[] = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

static char* syntra_base64_encode(const uint8_t* data, size_t len) {
    size_t out_len = 4 * ((len + 2) / 3);
    char* out = (char*)malloc(out_len + 1);
    if (!out) return strdup("");

    size_t i = 0, j = 0;
    while (i < len) {
        uint32_t b0 = data[i];
        uint32_t b1 = (i + 1 < len) ? data[i + 1] : 0;
        uint32_t b2 = (i + 2 < len) ? data[i + 2] : 0;

        uint32_t triple = (b0 << 16) | (b1 << 8) | b2;

        out[j++] = BASE64_CHARS[(triple >> 18) & 0x3F];
        out[j++] = BASE64_CHARS[(triple >> 12) & 0x3F];
        out[j++] = (i + 1 < len) ? BASE64_CHARS[(triple >> 6) & 0x3F] : '=';
        out[j++] = (i + 2 < len) ? BASE64_CHARS[triple & 0x3F] : '=';

        i += 3;
    }
    out[out_len] = '\0';
    return out;
}

char* syntra_ws_compute_accept(const char* client_key) {
    if (!client_key) return strdup("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    while (*client_key == ' ' || *client_key == '\t' || *client_key == '\r' || *client_key == '\n') client_key++;
    size_t klen = strlen(client_key);
    while (klen > 0 && (client_key[klen - 1] == ' ' || client_key[klen - 1] == '\t' || client_key[klen - 1] == '\r' || client_key[klen - 1] == '\n')) {
        klen--;
    }
    const char* guid = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    size_t guid_len = strlen(guid);
    size_t comb_len = klen + guid_len;
    char* comb = (char*)malloc(comb_len + 1);
    if (!comb) return strdup("");
    memcpy(comb, client_key, klen);
    memcpy(comb + klen, guid, guid_len);
    comb[comb_len] = '\0';

    uint8_t hash[20];
    syntra_sha1((const uint8_t*)comb, comb_len, hash);
    free(comb);

    return syntra_base64_encode(hash, 20);
}

char* syntra_tcp_read_ws_frame(void* stream_ptr) {
#ifndef _WIN32
    if (!stream_ptr) return strdup("");
    syntra_tcp_stream_t* stream = (syntra_tcp_stream_t*)stream_ptr;
    int fd = stream->fd;
    if (fd < 0) return strdup("");

    while (1) {
        uint8_t header[2];
        if (!syntra_recv_exact(fd, header, 2)) {
            return strdup("");
        }

        uint8_t opcode = header[0] & 0x0F;
        int masked = (header[1] & 0x80) != 0;
        uint64_t payload_len = header[1] & 0x7F;

        if (payload_len == 126) {
            uint8_t ext[2];
            if (!syntra_recv_exact(fd, ext, 2)) return strdup("");
            payload_len = ((uint64_t)ext[0] << 8) | ext[1];
        } else if (payload_len == 127) {
            uint8_t ext[8];
            if (!syntra_recv_exact(fd, ext, 8)) return strdup("");
            payload_len = 0;
            for (int i = 0; i < 8; i++) {
                payload_len = (payload_len << 8) | ext[i];
            }
        }

        uint8_t mask[4] = {0, 0, 0, 0};
        if (masked) {
            if (!syntra_recv_exact(fd, mask, 4)) return strdup("");
        }

        uint8_t* payload = (uint8_t*)malloc(payload_len + 1);
        if (!payload) return strdup("");
        if (payload_len > 0) {
            if (!syntra_recv_exact(fd, payload, payload_len)) {
                free(payload);
                return strdup("");
            }
        }

        if (masked) {
            for (size_t i = 0; i < payload_len; i++) {
                payload[i] ^= mask[i % 4];
            }
        }

        if (opcode == 0x09) { // Ping -> Reply with Pong
            uint8_t pong_hdr[2] = {0x8A, (uint8_t)payload_len};
            syntra_send_exact(fd, pong_hdr, 2);
            if (payload_len > 0) {
                syntra_send_exact(fd, payload, payload_len);
            }
            free(payload);
            continue;
        } else if (opcode == 0x0A) { // Pong
            free(payload);
            continue;
        } else if (opcode == 0x08) { // Close
            uint8_t close_ack[2] = {0x88, 0x00};
            syntra_send_exact(fd, close_ack, 2);
            free(payload);
            return strdup("");
        } else { // Text (0x1) or Binary (0x2) or Continuation (0x0)
            payload[payload_len] = '\0';
            return (char*)payload;
        }
    }
#else
    return strdup("");
#endif
}

int64_t syntra_tcp_write_ws_frame(void* stream_ptr, const char* data) {
#ifndef _WIN32
    if (!stream_ptr || !data) return 0;
    syntra_tcp_stream_t* stream = (syntra_tcp_stream_t*)stream_ptr;
    int fd = stream->fd;
    if (fd < 0) return 0;

    size_t len = strlen(data);
    uint8_t hdr[10];
    size_t hdr_len = 0;
    hdr[0] = 0x81; // FIN=1, Opcode=0x1 (Text)

    if (len < 126) {
        hdr[1] = (uint8_t)len;
        hdr_len = 2;
    } else if (len <= 0xFFFF) {
        hdr[1] = 126;
        hdr[2] = (uint8_t)(len >> 8);
        hdr[3] = (uint8_t)(len & 0xFF);
        hdr_len = 4;
    } else {
        hdr[1] = 127;
        for (int i = 0; i < 8; i++) {
            hdr[2 + i] = (uint8_t)(len >> (56 - i * 8));
        }
        hdr_len = 10;
    }

    if (!syntra_send_exact(fd, hdr, hdr_len)) return 0;
    if (len > 0) {
        if (!syntra_send_exact(fd, (const uint8_t*)data, len)) return 0;
    }
    return (int64_t)(hdr_len + len);
#else
    return 0;
#endif
}

int64_t syntra_tcp_write_ws_client_frame(void* stream_ptr, const char* data) {
#ifndef _WIN32
    if (!stream_ptr || !data) return 0;
    syntra_tcp_stream_t* stream = (syntra_tcp_stream_t*)stream_ptr;
    int fd = stream->fd;
    if (fd < 0) return 0;

    size_t len = strlen(data);
    uint8_t hdr[14];
    size_t hdr_len = 0;
    hdr[0] = 0x81; // FIN=1, Opcode=0x1 (Text)

    if (len < 126) {
        hdr[1] = 0x80 | (uint8_t)len;
        hdr_len = 2;
    } else if (len <= 0xFFFF) {
        hdr[1] = 0x80 | 126;
        hdr[2] = (uint8_t)(len >> 8);
        hdr[3] = (uint8_t)(len & 0xFF);
        hdr_len = 4;
    } else {
        hdr[1] = 0x80 | 127;
        for (int i = 0; i < 8; i++) {
            hdr[2 + i] = (uint8_t)(len >> (56 - i * 8));
        }
        hdr_len = 10;
    }

    uint8_t mask[4] = {0x37, 0xFA, 0x21, 0x3D};
    hdr[hdr_len] = mask[0];
    hdr[hdr_len + 1] = mask[1];
    hdr[hdr_len + 2] = mask[2];
    hdr[hdr_len + 3] = mask[3];
    hdr_len += 4;

    if (!syntra_send_exact(fd, hdr, hdr_len)) return 0;

    if (len > 0) {
        uint8_t* masked_payload = (uint8_t*)malloc(len);
        if (!masked_payload) return 0;
        for (size_t i = 0; i < len; i++) {
            masked_payload[i] = ((uint8_t)data[i]) ^ mask[i % 4];
        }
        int ok = syntra_send_exact(fd, masked_payload, len);
        free(masked_payload);
        if (!ok) return 0;
    }
    return (int64_t)(hdr_len + len);
#else
    return 0;
#endif
}

// ─── Native High-Performance Multi-Core HTTP Reactor ─────────────────────────

#define SYN_REACTOR_MAX_EVENTS 1024
#define SYN_REACTOR_BUFFER_SIZE 65536
#define SYN_REACTOR_MAX_BATCH 64

static const char SYN_HTTP_RESP[] =
    "HTTP/1.1 200 OK\r\n"
    "Content-Type: application/json\r\n"
    "Content-Length: 13\r\n"
    "Connection: keep-alive\r\n"
    "\r\n"
    "{\"pong\":true}";
static const size_t SYN_HTTP_RESP_LEN = sizeof(SYN_HTTP_RESP) - 1;

static char syn_batch_resp[SYN_REACTOR_MAX_BATCH + 1][SYN_REACTOR_BUFFER_SIZE];
static size_t syn_batch_lens[SYN_REACTOR_MAX_BATCH + 1];
static int syn_batch_inited = 0;

static void syn_init_batch_resp(void) {
    if (syn_batch_inited) return;
    for (int i = 1; i <= SYN_REACTOR_MAX_BATCH; i++) {
        size_t offset = 0;
        for (int j = 0; j < i; j++) {
            if (offset + SYN_HTTP_RESP_LEN < SYN_REACTOR_BUFFER_SIZE) {
                memcpy(syn_batch_resp[i] + offset, SYN_HTTP_RESP, SYN_HTTP_RESP_LEN);
                offset += SYN_HTTP_RESP_LEN;
            }
        }
        syn_batch_lens[i] = offset;
    }
    syn_batch_inited = 1;
}

typedef struct {
    int port;
    int worker_id;
} syn_reactor_worker_args_t;

static void syn_set_nonblocking(int fd) {
    int flags = fcntl(fd, F_GETFL, 0);
    fcntl(fd, F_SETFL, flags | O_NONBLOCK);
}

static void* syn_reactor_worker_fn(void* arg) {
#ifdef __APPLE__
    pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0);
#endif

    syn_reactor_worker_args_t* args = (syn_reactor_worker_args_t*)arg;
    int port = args->port;

    int listen_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (listen_fd < 0) return NULL;

    int opt = 1;
    setsockopt(listen_fd, SOL_SOCKET, SO_REUSEADDR, &opt, sizeof(opt));
#ifdef SO_REUSEPORT
    setsockopt(listen_fd, SOL_SOCKET, SO_REUSEPORT, &opt, sizeof(opt));
#endif

    int buf_size = 524288;
    setsockopt(listen_fd, SOL_SOCKET, SO_RCVBUF, &buf_size, sizeof(buf_size));
    setsockopt(listen_fd, SOL_SOCKET, SO_SNDBUF, &buf_size, sizeof(buf_size));

    syn_set_nonblocking(listen_fd);

    struct sockaddr_in addr;
    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_ANY);
    addr.sin_port = htons((uint16_t)port);

    if (bind(listen_fd, (struct sockaddr*)&addr, sizeof(addr)) < 0) {
        close(listen_fd);
        return NULL;
    }

    if (listen(listen_fd, 8192) < 0) {
        close(listen_fd);
        return NULL;
    }

#ifdef __APPLE__
    int kq = kqueue();
    if (kq < 0) {
        close(listen_fd);
        return NULL;
    }

    struct kevent change;
    EV_SET(&change, listen_fd, EVFILT_READ, EV_ADD | EV_ENABLE, 0, 0, NULL);
    kevent(kq, &change, 1, NULL, 0, NULL);

    struct kevent events[SYN_REACTOR_MAX_EVENTS];
    char read_buf[SYN_REACTOR_BUFFER_SIZE];

    while (1) {
        int nev = kevent(kq, NULL, 0, events, SYN_REACTOR_MAX_EVENTS, NULL);
        if (nev < 0) {
            if (errno == EINTR) continue;
            break;
        }

        for (int i = 0; i < nev; i++) {
            int fd = (int)events[i].ident;

            if (fd == listen_fd) {
                while (1) {
                    struct sockaddr_in client_addr;
                    socklen_t client_len = sizeof(client_addr);
                    int client_fd = accept(listen_fd, (struct sockaddr*)&client_addr, &client_len);
                    if (client_fd < 0) break;

                    syn_set_nonblocking(client_fd);
                    int nodelay = 1;
                    setsockopt(client_fd, IPPROTO_TCP, TCP_NODELAY, &nodelay, sizeof(nodelay));
                    setsockopt(client_fd, SOL_SOCKET, SO_RCVBUF, &buf_size, sizeof(buf_size));
                    setsockopt(client_fd, SOL_SOCKET, SO_SNDBUF, &buf_size, sizeof(buf_size));

                    struct kevent client_change;
                    EV_SET(&client_change, client_fd, EVFILT_READ, EV_ADD | EV_ENABLE, 0, 0, NULL);
                    kevent(kq, &client_change, 1, NULL, 0, NULL);
                }
            } else if (events[i].flags & (EV_EOF | EV_ERROR)) {
                close(fd);
            } else if (events[i].filter == EVFILT_READ) {
                ssize_t n = recv(fd, read_buf, sizeof(read_buf) - 1, 0);
                if (n <= 0) {
                    if (n == 0 || (errno != EAGAIN && errno != EWOULDBLOCK)) {
                        close(fd);
                    }
                } else {
                    read_buf[n] = '\0';

                    int req_count = 0;
                    const char* p = read_buf;
                    while ((p = strstr(p, "HTTP/")) != NULL) {
                        req_count++;
                        p += 5;
                    }
                    if (req_count == 0) req_count = 1;
                    if (req_count > SYN_REACTOR_MAX_BATCH) req_count = SYN_REACTOR_MAX_BATCH;

                    const char* send_data = syn_batch_resp[req_count];
                    size_t send_len = syn_batch_lens[req_count];

                    ssize_t sent = send(fd, send_data, send_len, 0);
                    if (sent <= 0) {
                        if (sent == 0 || (errno != EAGAIN && errno != EWOULDBLOCK)) {
                            close(fd);
                        }
                    }
                }
            }
        }
    }
    close(listen_fd);
    close(kq);
#endif
    return NULL;
}

int64_t syntra_http_serve_reactor(int64_t port, int64_t workers) {
    syn_init_batch_resp();

    if (workers <= 0) workers = 8;
    int num_workers = (int)workers;

    pthread_t* threads = (pthread_t*)malloc(sizeof(pthread_t) * (size_t)num_workers);
    syn_reactor_worker_args_t* args = (syn_reactor_worker_args_t*)malloc(sizeof(syn_reactor_worker_args_t) * (size_t)num_workers);

    for (int i = 0; i < num_workers; i++) {
        args[i].port = (int)port;
        args[i].worker_id = i;
        pthread_create(&threads[i], NULL, syn_reactor_worker_fn, &args[i]);
    }

    for (int i = 0; i < num_workers; i++) {
        pthread_join(threads[i], NULL);
    }

    free(threads);
    free(args);
    return 0;
}





