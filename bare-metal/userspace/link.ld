/* Privion OS userspace linker script
 * Compatible with seL4 rootserver and child process loading.
 *
 * Load address 0x400000 is conventional for x86_64 ELF executables and
 * leaves the lower 4 MiB free (which seL4 reserves for kernel mappings).
 *
 * This script is used for all Privion userspace binaries. It is referenced
 * by userspace/init/build.rs (and future build.rs files for other services).
 */

ENTRY(_sel4_start)

SECTIONS {
    . = 0x400000;

    /* ------------------------------------------------------------------ */
    /* Code                                                                 */
    /* ------------------------------------------------------------------ */
    .text ALIGN(0x1000) : {
        /* Ensure _start (the entry point) is placed first */
        *(.text._start)
        *(.text.start)
        *(.text .text.*)
    }

    /* ------------------------------------------------------------------ */
    /* Read-only data                                                       */
    /* ------------------------------------------------------------------ */
    .rodata ALIGN(0x1000) : {
        *(.rodata .rodata.*)
    }

    /* ------------------------------------------------------------------ */
    /* Read-write data (initialised)                                        */
    /* ------------------------------------------------------------------ */
    .data ALIGN(0x1000) : {
        *(.data .data.*)
    }

    /* ------------------------------------------------------------------ */
    /* Thread-Local Storage                                                 */
    /* Required by sel4runtime for the per-thread IPC buffer pointer.      */
    /* ------------------------------------------------------------------ */
    .tdata ALIGN(0x1000) : {
        _tdata_start = .;
        *(.tdata .tdata.*)
        _tdata_end = .;
    }

    .tbss ALIGN(0x1000) : {
        _tbss_start = .;
        *(.tbss .tbss.*)
        _tbss_end = .;
    }

    /* ------------------------------------------------------------------ */
    /* Uninitialised data (zero on load)                                   */
    /* ------------------------------------------------------------------ */
    .bss ALIGN(0x1000) (NOLOAD) : {
        _bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        _bss_end = .;
    }

    _end = .;

    /* ------------------------------------------------------------------ */
    /* Discard debug/unwind info from the final binary                     */
    /* ------------------------------------------------------------------ */
    /DISCARD/ : {
        *(.eh_frame*)
        *(.note*)
        *(.comment)
    }
}
