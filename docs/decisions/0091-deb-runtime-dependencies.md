# 0091 — The DEB declares the libraries the desktop binary loads at run time

Date: 2026-11-11 · Ticket: [T27](../tasks/T27-cross-platform-release.md) · Status: accepted

## Context

The release pipeline's DEB install verification reached its GUI
launch stage on 2026-11-11 (run
`85502100-799b-42fe-b1b4-352488538449`, commit `4273471`) and the
desktop binary died instead of starting:

```
thread 'main' panicked at xkbcommon-dl-0.4.2/src/x11.rs:59:28:
Library libxkbcommon-x11.so could not be loaded.
FAIL: the desktop binary exited with status 101 instead of staying up
```

The cause was a missing library in the verification environment, and
the same gap existed in the package itself. `libxkbcommon0` does not
carry the X11 half of libxkbcommon; `libxkbcommon-x11-0` does. The CI
image installed only the former.

The package was the bigger instance of the same defect.
`packaging/deb/control` declared `Depends: libc6 (>= 2.36)` and
nothing else, while the desktop binary needs the X11 and GL stack to
start. Decision 0077 recorded that as a separate packaging gap: "The
`DEB` still declares only `libc6`."

Nothing could have caught it by inspection. The binary's dynamic
section does not mention the X11 stack at all:

```
$ objdump -p firecraft | grep NEEDED
  NEEDED  libgcc_s.so.1
  NEEDED  libm.so.6
  NEEDED  libc.so.6
  NEEDED  ld-linux-x86-64.so.2
```

`x11-dl` and `xkbcommon-dl` load their libraries with `dlopen`, so
`dpkg-shlibdeps` cannot see them, the ELF dynamic section cannot see
them, and the dependency has to be declared by hand or not at all.

## Decision

- **The control file declares the run-time set.** `Depends:` names
  `libgcc-s1`, `libgl1`, `libx11-6`, `libxcursor1`, `libxrandr2`,
  `libxi6`, `libxkbcommon0` and `libxkbcommon-x11-0` alongside the
  existing `libc6 (>= 2.36)` floor.
- **The build refuses to ship a package that drops one.**
  `xtask`'s `build_deb` reads the control file and fails when any of
  those libraries is undeclared, naming what is missing. The list
  lives in one place (`GUI_RUNTIME_LIBRARIES` in `xtask/src/dist.rs`),
  and the gate reads the shipped control file, so editing the control
  without the gate fails the release build.
- **The verification installs exactly what the package declares.**
  The DEB install stage installs the declared set plus its harness
  (`xvfb`, `xauth`, `desktop-file-utils`) and nothing else, so a green
  launch is evidence that the declared dependencies are sufficient and
  not merely that the image happened to have the libraries.

## Evidence

Measured on 2026-11-11 with the binary built from `4273471`, in clean
`debian:12-slim` containers, one variable at a time:

| Libraries present | Result |
|---|---|
| the CI's set, **without** `libxkbcommon-x11-0` | panic, `libxkbcommon-x11.so could not be loaded`, exit 101 — the pipeline's failure, reproduced |
| the CI's set **with** `libxkbcommon-x11-0` | launched and stayed up for the 25 s window, exit 124, empty log |
| `xvfb xauth libxkbcommon-x11-0` only | `XNotSupported(LibraryOpenError(... libXcursor.so.1: cannot open shared object file ...))`, exit 1 |
| the X11 stack without `libgl1` | launched and stayed up, exit 124 |

`libgl1` is declared even though the launch does not need it: the
desktop binary renders through OpenGL, and a user session that cannot
provide it would get a window that never draws.

## Consequences

- The DEB now installs the libraries it needs, and `dpkg` refuses it
  on a system that cannot provide them — a loud failure at install
  time rather than a window that never opens.
- The release pipeline's GUI launch stage stops failing on the missing
  library. The stage had never passed before: the DEB verification
  landed after the last fully green run, so this is its first green
  launch, not a recovery.
- The `Depends` field is now longer than one line's worth of
  packages, and `xtask`'s libc6 floor reader still parses it: it
  splits on `libc6 (>= ` and takes the text up to the closing
  parenthesis.
- The published v0.2.5 package is historical and is not replaced; its
  `Depends` stays `libc6 (>= 2.31)`, recording what was published.

See also decision 0077 (the libc6 floor) and 0086 (the pipeline fails
closed).
