/* A program that leaks on purpose, from two distinct call sites, so memleak
 * can group outstanding allocations by stack.
 *   build on the VM:  clang -O0 -g -fno-omit-frame-pointer -o leaker leaker.c
 * -fno-omit-frame-pointer keeps the user stack walkable by bpf_get_stackid. */
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

static void *leak_here(size_t n) { return malloc(n); }      /* never freed */
static void  use_and_free(size_t n) { void *p = malloc(n); free(p); }  /* fine */

int main(void) {
    fprintf(stderr, "leaker pid %d\n", getpid());
    for (long i = 0; ; i++) {
        use_and_free(128);          /* balanced */
        if (i % 4 == 0) leak_here(4096);   /* leaks 4 KiB every 4th iter */
        usleep(20000);              /* 50/sec */
    }
    return 0;
}
