# Zenodo Paper

The PST OS research paper is available in the repository at `paper/pst-os.tex`.

## Title

**Parallel String Theory: Replacing Hierarchical Data Structures with Positional Identity and Constraint Solving**

## Abstract

We present Parallel String Theory, a systems architecture that replaces hierarchical data structures with flat parallel strings and constraint-based relationships. PST OS, a 217KB operating system built on the formally verified seL4 microkernel, proves this claim empirically: it boots to a windowed desktop with keyboard input, persistence, networking, and a live Markout shell, without a single tree or pointer graph in its architecture.

## Building the PDF

```bash
cd paper
pdflatex pst-os.tex
bibtex pst-os
pdflatex pst-os.tex
pdflatex pst-os.tex
```

## Sections

1. Introduction — why trees aren't necessary
2. The Parallel Strings Primitive — definition, invariants, complexity
3. OS Subsystems — process table, VFS, IPC, scheduler, memory, time
4. Markout — unifying temporal and spatial constraints
5. Implementation — seL4, Rust no_std, drivers, applications
6. Evaluation — performance, cross-domain validation, convergence
7. Related Work — column stores, LSM trees, Cassowary, React
8. Discussion — strengths, limitations, future work
9. Conclusion

## Citation

The paper will be published on Zenodo with a DOI for academic citation.
