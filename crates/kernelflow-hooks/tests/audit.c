
#define _GNU_SOURCE
#include <link.h>
#include <stdio.h>
#include <string.h>

unsigned int la_version(unsigned int v) {
   printf("*************LD_AUDIT\n");
   fflush(stdout);


    return v; }


ElfW(Addr)
la_pltenter64(ElfW(Sym) *sym, unsigned int, uintptr_t *, uintptr_t *,
            unsigned int *, const char *name, long int *)
{
    if (strstr(name, "cu"))
        fprintf(stderr, "call → %s\n", name);
    printf("call → %s\n", name);
    fflush(stdout);
    return sym->st_value;          /* let the call proceed unmodified */
}
