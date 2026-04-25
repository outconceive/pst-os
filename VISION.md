\# The Universal UI Primitive

\## Outconceive / PST OS — Vision Document



\---



\## The Problem



Every platform has its own UI toolkit.



iOS has SwiftUI. Android has Jetpack Compose. The web has React. The desktop has Electron pretending to be native. Terminals have Ink. Embedded systems have nothing at all.



They are all solving the same problem — \*describe what the user sees and how it responds\* — with incompatible answers. A developer who wants to reach every surface writes the same app four times, in four languages, against four frameworks, maintaining four codebases.



The "write once, run anywhere" promise has been broken so many times it stopped being made. Electron ships a browser to avoid the problem. React Native papers over it. Flutter draws everything itself and calls it solved.



None of them fixed the root cause.



\---



\## The Root Cause



Every existing UI framework models the interface as a \*\*tree\*\*.



Components own children. Identity requires generated keys. Structural changes cascade through the hierarchy. The tree is the assumption so baked into the model that nobody questions it.



But the tree is not necessary. It is the first solution that worked, and it calcified into convention.



\---



\## The Insight



\*\*Position is identity.\*\*



A component's identity is its location in the source — its line number and character offset. Not a generated key. Not a pointer. Not a path through a hierarchy. Just a position.



This is the parallel strings model. Each UI row is four equal-length strings:



```

content:    "Username  \_\_\_\_\_\_\_\_  Login "

components: "LLLLLLLLLLIIIIIIIIIIBBBBBB"

state\_keys: "\_\_\_\_\_\_\_\_\_\_username\_\_submit"

styles:     "                    pppppp"

```



A component is a column index across all strings. Spawning appends. Killing tombstones. The position never gets reused. There is no tree. There is no reconciliation. State updates re-render only the affected lines: O(1).



\---



\## The Language



\*\*Markout\*\* is the declarative UI language that expresses this model.



```

@card padding:24

| Welcome back, {label:user animate:fade}

| Email     {input:email validate:required,email col-6}

| Password  {password:pass validate:required,min:8 col-6}

| {button:login "Sign In" primary}  {spacer:end}  {link:forgot "Forgot password?" ghost}

@end card

```



No JSX. No transpiler. No node\_modules. No build step.



The `@parametric` container extends this to constraint-based layout — components anchor to each other rather than to a grid:



```

@parametric

| {label:title "Dashboard"}

| {input:search center-x:title gap-y:1rem}

| {button:go "Search" after:search gap-x:8px center-y:search}

@end parametric

```



`center-y:search` means \*my vertical center aligns with search's vertical center\*. The solver computes absolute positions from these relationships. No coordinates. No CSS. No runtime measurement.



\---



\## The Solver



The parametric constraint solver is a topological sort over a dependency graph.



It takes declared relationships — spatial in the UI, temporal in the OS scheduler — and computes the positions that satisfy them. The solver does not know or care what output surface it is targeting. It produces a resolved VNode tree with absolute coordinates.



The renderer decides what those coordinates mean.



\---



\## The Stack



```

Developer writes Markout

&#x20;       │

&#x20;       ▼

pst-markout parser

&#x20;       │

&#x20;       ▼

Parametric constraint solver

&#x20;       │

&#x20;       ▼

Resolved VNode tree

&#x20;       │

&#x20;       ├──→ html::to\_html      → Browser DOM      (Outconceive web)

&#x20;       ├──→ pst-framebuffer    → VGA pixels        (PST OS desktop)

&#x20;       ├──→ terminal renderer  → ANSI cells        (SSH / terminal)

&#x20;       └──→ embedded renderer  → raw framebuffer   (no OS required)

```



One data structure. One algorithm. One language. One solver. N renderers.



Each renderer is a thin output adapter. It receives positions and VNodes and paints them onto whatever surface it has. The solver does not leak into the renderer. The renderer does not reach back into the solver.



This is why it works where every other "universal" framework failed. They built N abstractions trying to hide N platforms. This builds one primitive that N platforms consume.



\---



\## The OS



PST OS is the proof that the parallel strings principle generalizes beyond UI.



The same model — position is identity, append-only mutation, constraint-solved ordering — applies to every OS subsystem:



| Subsystem | Parallel strings | Constraint |

|-----------|-----------------|------------|

| Process table | state, affinity, owner, privilege | spawn order |

| Filesystem | name, content, owner, flags | prefix scan |

| Scheduler | dependencies, deadlines, rates | topological sort |

| Memory | start, length, owner, status | coalescing |

| IPC | payload, sender, receiver, status | delivery order |

| Time | tick, delta, retention, compaction | temporal ordering |



The kernel is a pure-math constraint solver. The hardware contract is two immortal positions: the bootloader jump and the offset table root. Everything else is ephemeral, append-only parallel strings, lazily compacted over time.



The desktop environment is the top-level Markout document. Every app is a `@parametric` block. Moving a window is editing a constraint. The compositor is the renderer. There is no display server.



Drivers are hardware Markout:



```

@peripheral:nic interface:eth0

| {dma\_channel:rx privilege:hw-write}

| {constraint:rate "N bytes/tick"}

| {constraint:trigger after:dma-complete gap:2ms}

@end peripheral

```



The same syntax that declares a login form declares a network interface. One language. All the way down.



\---



\## The Privacy Guarantee



PST OS is built on seL4 — the only formally verified microkernel in production use. Every component runs in capability-isolated userspace. A bug in the network stack cannot compromise the filesystem. A bug in the display driver cannot read process memory.



Privacy is structural, not configured. Zero telemetry is not a setting. It is an architectural consequence of the capability model. The OS cannot exfiltrate data it cannot access. Applications cannot access what they have not been explicitly granted.



The append-only time dimension means the audit trail is free. Every state change is recorded by definition. `forget\_all()` on shutdown is amnesic mode — the privacy guarantee Tails offers, but on a platform you can also develop on.



\---



\## The Vision



Markout becomes the universal UI language.



A developer writes one thing. It runs on the browser, the desktop, the terminal, embedded hardware. The constraint solver handles layout on every surface. The privacy guarantee travels with the code.



The gap between "web app" and "native app" closes. Not because everything becomes a browser — because everything shares a rendering primitive that is lighter than a browser, more expressive than a terminal, and more portable than any native toolkit.



The gap between "OS developer" and "app developer" closes. Writing an app for PST OS is writing Markout. The OS and the app speak the same language.



This is what becomes possible when you question the assumption everyone else is standing on.



The tree was never necessary.



\---



\## Current Status



| Component | Status |

|-----------|--------|

| Outconceive web framework | ✅ Shipped — parallel strings, Markout, @parametric, visual IDE |

| PST OS core crates (137+ tests) | ✅ Green — all subsystems |

| Bare metal seL4 boot | ✅ Done — Markout renders on serial from cold boot |

| VGA framebuffer | ✅ Done — 2MB large page mapped, desktop on screen |

| Terminal renderer | ✅ Done — pst-terminal crate, Markout → ANSI |

| Keyboard input | ✅ Done — PS/2 IRQ via IOAPIC |

| Markout shell | ✅ Done — type Markout, render live |

| Multiple windows | ✅ Done — Tab focus, status bar |

| Text editor + Code stepper | ✅ Done — .txt/.md word processor, syntax highlighting |

| dt:// and gh:// browser | ✅ Done — Markout pages from disk and GitHub |

| Persistence + Network | ✅ Done — virtio-blk, virtio-net, smoltcp TCP/IP |

| Outconceive convergence | ✅ Done — same document renders on browser, terminal, and bare metal |



\---



\*One primitive. One loop. One language. Every surface.\*

