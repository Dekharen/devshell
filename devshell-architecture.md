# Devshell – Architecture & Design

## Overview

Devshell is a lightweight CLI tool for composing and running developer
containers using Dockerfile fragments.

Its core philosophy is:

- file-system driven
- Docker-native
- minimal opinionated logic
- full transparency

All complexity lives in Dockerfile fragments, not in the tool.

---

## Configuration Resolution

The main entrypoint is:

```bash
    devshell run [config-name]
```

Resolution order:

1. If `config-name` is provided:
   - Resolve `~/.config/devshell/<config-name>.devshell.toml`
   - Error if not found

2. If no name is provided:
   - Look for `*.devshell.toml` in the current directory
   - If exactly one is found → use it
   - If more than one is found → error

3. Fallback:
   - Use `~/.config/devshell/default.devshell.toml`
   - Error if missing

This resolution strategy is deterministic and explicit.

---

## TOML Configuration Format

Example:

```toml
    name = "dev-shell"

    stages = [
      "base/debian",
      "features/rust",
      "features/neovim",
      "post/dev-user",
    ]

    docker_args = [
      "--memory=8g",
      "--cpus=6",
    ]

    [[volumes]]
    host = "/home/user/projects"
    container = "/workspace"
    mode = "rw"

    [[volumes]]
    host = "/home/user/.ssh"
    container = "/home/dev/.ssh"
    mode = "ro"
```

### Fields

- `name`  
  Human-readable name (used for container/image naming)

- `stages` (required)  
  Ordered list of Dockerfile fragment references

- `docker_args` (optional)  
  Additional arguments passed verbatim to `docker run`

- `volumes` (optional)  
  Structured volume definitions. Each `[[volumes]]` entry is additive;
  multiple volumes are mounted together, not replaced.

---

## Dockerfile Fragments

Fragments are plain Dockerfile snippets.

### Filesystem layout

    ~/.local/share/devshell/fragments/
    ├── base/
    │   ├── debian.docker
    │   └── fedora.docker
    ├── features/
    │   ├── rust.docker
    │   ├── neovim.docker
    │   └── bevy.docker
    └── post/
        └── dev-user.docker

Only files ending in `.docker` are considered.

### Resolution

A stage reference:

    features/rust

Resolves to:

```
~/.local/share/devshell/fragments/features/rust.docker
```

If a fragment does not exist:

- a clear error is emitted
- Docker is not invoked

---

## Embedded Fragments

Some fragments are embedded directly in the binary.

They are referenced using the `@` prefix:

    @base/debian

Embedded fragments:

- are not visible on disk by default
- can be inspected via `devshell show`
- can be materialized via `devshell generate`

---

## CLI Commands

### devshell run [name]

- Resolves configuration
- Generates Dockerfile
- Builds image
- Runs container
- Exits when container exits

### devshell show <fragment>

Prints the resolved Dockerfile fragment.

Examples:

    devshell show base/debian
    devshell show @features/rust

### devshell generate <fragment | all>

Materializes embedded fragments to disk.

Examples:

    devshell generate @base/debian
    devshell generate all

---

## Docker Execution Model

1. Concatenate fragments → Dockerfile
2. Build image
3. Run container with:
   - docker_args
   - volumes
4. Devshell does not daemonize
5. No long-running background state

---

## Design Goals

- Zero custom DSL
- Dockerfile-first
- Easy to debug
- Easy to extend via files
- Predictable behavior

---

## Non-Goals

- No orchestration
- No container lifecycle management
- No dependency resolution
- No remote execution (for now)

---

## Summary

Devshell is a thin, deterministic layer over Docker that enables
composable, inspectable, and portable developer environments.

The filesystem *is* the configuration.

# Devshell – Rust Crate / Module Layout and Error Model

## Embedded Fragments

The executable ships with a minimal set of embedded Docker fragments. These fragments behave exactly like on-disk fragments and can be referenced using the @ prefix.

Embedded fragments provided in V1:

- base/debian
- base/ubuntu
- base/fedora

- features/basic-debian
- features/basic-ubuntu
- features/basic-fedora

- post/dev-user

Each embedded fragment is a plain Dockerfile fragment. No templating, no variables, no logic.

Users may generate these fragments to disk using:

~~~text
devshell generate all
~~~

or inspect them using:

~~~text
devshell show base/debian
~~~

Resolution order for fragments:

1. On-disk fragment in fragments directory
2. Embedded fragment
3. Error (fragment not found)

## Rust Crate Layout

The crate is intentionally small and flat. The goal is orchestration, not abstraction.

Proposed layout:

~~~text
devshell/
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── config/
│   │   ├── mod.rs
│   │   ├── load.rs
│   │   ├── schema.rs
│   ├── fragments/
│   │   ├── mod.rs
│   │   ├── embedded.rs
│   │   ├── resolve.rs
│   ├── docker/
│   │   ├── mod.rs
│   │   ├── build.rs
│   │   ├── run.rs
│   ├── fs.rs
│   ├── error.rs
│   └── util.rs
└── Cargo.toml
~~~

### main.rs

- Entry point
- Initializes logging
- Dispatches to CLI handling

### cli.rs

- clap definitions
- Commands:
  - run
  - build
  - show
  - generate
  - doctor
- Resolves config name and working directory

### config/schema.rs

- TOML structs
- Minimal parsing only
- Fields:
  - name
  - stages (ordered list)
  - docker_args (list of strings)
  - volumes (list of source:target strings)

### config/load.rs

- Config resolution logic:
  1. Named config in global config directory
  2. Local *.devshell.toml (error if multiple)
  3. Global default.toml

### fragments/embedded.rs

- Static string map of embedded fragments
- No logic
- Keys match fragment paths (base/debian)

### fragments/resolve.rs

- Resolves fragment references:
  - @fragment
  - fragment
- Returns ordered list of fragment contents
- Emits warnings for missing fragments

### docker/build.rs

- Concatenates fragments
- Writes temporary Dockerfile
- Executes docker build

### docker/run.rs

- Executes docker run
- Applies:
  - docker_args
  - volume mounts
- Handles attach vs exec semantics later

### fs.rs

- All filesystem access
- XDG paths
- Fragment discovery

### error.rs

- Central error enum
- Human-readable errors
- No panic paths in normal execution

## Error Model

Errors are explicit, early, and fatal unless explicitly marked as warnings.

### Hard Errors

- Named config not found
- Multiple local *.devshell.toml files
- No valid config found
- Docker binary missing
- Docker daemon not running
- Invalid TOML syntax
- Invalid volume specification
- Docker build failure
- Docker run failure

### Soft Errors (Warnings)

- Missing fragment (skipped but reported)
- Embedded fragment overridden by disk fragment
- Unused fragments

### Doctor Command

~~~text
devshell doctor
~~~

Checks:

- Docker availability
- Config resolution
- Fragment existence
- Embedded vs disk overrides

## Design Constraints Recap

- No Dockerfile DSL
- No fragment logic
- No conditional execution
- No implicit defaults beyond resolution order
- Filesystem and Docker are the source of truth

The tool concatenates files, runs Docker, and gets out of the way.
