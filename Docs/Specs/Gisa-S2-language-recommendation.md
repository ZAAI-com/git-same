# Language & Framework Recommendation

## Evaluation Criteria

For Gisa, the ideal language should excel at:

1. **CLI experience** — Argument parsing, help generation, shell completion
2. **Concurrency** — Parallel HTTP requests and git operations
3. **Distribution** — Easy installation, minimal dependencies
4. **HTTP client** — GitHub API integration with proper error handling
5. **Process spawning** — Running git commands reliably
6. **Cross-platform** — macOS primary, Linux/Windows secondary
7. **Developer velocity** — Time to functional prototype

## Language Comparison

### Rust

**Ecosystem**:
- CLI: `clap` (best-in-class argument parsing, derives, shell completions)
- HTTP: `reqwest` (async, well-maintained)
- Async: `tokio` (mature runtime)
- Progress: `indicatif` (beautiful progress bars)
- Git: `git2` (libgit2 bindings) or shell out to `git`

**Pros**:
- Single static binary, no runtime needed
- Excellent performance, low memory footprint
- Strong type system catches bugs at compile time
- `clap` derives generate help, completions, and validation automatically
- Great error handling with `Result` and `?` operator
- Memory safety without garbage collection

**Cons**:
- Steeper learning curve
- Longer compile times during development
- More verbose than scripting languages
- `git2` (libgit2) can be tricky to compile; shelling out to `git` is often simpler

**Distribution**:
- `cargo install gisa`
- Homebrew formula (single binary)
- Pre-built binaries for all platforms

---

### Go

**Ecosystem**:
- CLI: `cobra` + `viper` (widely used, battle-tested)
- HTTP: `net/http` (stdlib) or `resty`
- Concurrency: goroutines + channels (built-in)
- Progress: `progressbar` or `mpb`
- Git: `go-git` (pure Go) or shell out

**Pros**:
- Single static binary
- Fast compilation
- Simple concurrency model (goroutines)
- `go-git` is pure Go, no C dependencies
- Straightforward to learn
- Excellent stdlib for HTTP/JSON

**Cons**:
- Error handling is verbose (`if err != nil`)
- Less expressive type system
- `cobra` requires more boilerplate than `clap`
- No sum types makes error states harder to model

**Distribution**:
- `go install github.com/user/gisa@latest`
- Homebrew
- Pre-built binaries

---

### TypeScript (Node.js)

**Ecosystem**:
- CLI: `commander`, `yargs`, or `oclif` (feature-rich)
- HTTP: `axios`, `undici`, or native `fetch`
- Concurrency: `Promise.all`, worker threads
- Progress: `ora`, `cli-progress`, `listr2`
- Git: `simple-git` (wrapper around git CLI)

**Pros**:
- Fastest development velocity
- Excellent async/await ergonomics
- Rich npm ecosystem
- Type safety with TypeScript
- `oclif` provides plugins, hooks, auto-updates

**Cons**:
- **Requires Node.js runtime** — Major friction for users
- Larger install size
- Startup time slower than native binaries
- Managing npm dependencies adds complexity

**Distribution**:
- `npm install -g gisa` (requires Node)
- Or bundle with `pkg`/`nexe` (larger binaries, ~50MB)

---

### Python

**Ecosystem**:
- CLI: `click` or `typer` (modern, type-hint based)
- HTTP: `httpx` (async) or `requests`
- Concurrency: `asyncio`, `concurrent.futures`
- Progress: `rich` (beautiful output), `tqdm`
- Git: `GitPython` or shell out

**Pros**:
- Very fast prototyping
- `typer` + `rich` create beautiful CLIs quickly
- Excellent for scripting operations
- Large community, many examples

**Cons**:
- **Requires Python runtime** — Version conflicts, venv complexity
- Slower execution than compiled languages
- Distribution is painful (PyInstaller, but large bundles)
- GIL limits true parallelism

**Distribution**:
- `pipx install gisa` (requires Python)
- PyInstaller bundles (~30-50MB)

---

### Swift

**Ecosystem**:
- CLI: `swift-argument-parser` (Apple's official)
- HTTP: `AsyncHTTPClient` or Foundation's URLSession
- Concurrency: Swift concurrency (async/await, actors)
- Progress: Limited options, would need custom or port
- Git: Shell out to `git`

**Pros**:
- Native on macOS, excellent integration
- Modern concurrency with async/await
- Good performance
- Single binary possible

**Cons**:
- **macOS-centric** — Cross-compilation is difficult
- Smaller CLI ecosystem
- Less battle-tested for this use case
- Linux support exists but is secondary

**Distribution**:
- Homebrew (macOS only realistically)
- Mint (`mint install user/gisa`)

---

### Deno (TypeScript)

**Ecosystem**:
- CLI: `cliffy` (clap-inspired), or `@std/cli`
- HTTP: Native `fetch`, `Deno.HttpClient`
- Concurrency: Native promises, workers
- Progress: `progress` module

**Pros**:
- Single executable runtime
- TypeScript native, no build step
- Better security model than Node
- Can compile to single binary (`deno compile`)
- Modern stdlib

**Cons**:
- Smaller ecosystem than Node
- Compiled binaries still large (~80MB)
- Less mature than established options
- Some npm packages don't work

**Distribution**:
- `deno install` from URL
- `deno compile` for standalone binary

---

## Comparison Matrix

| Criteria | Rust | Go | TypeScript | Python | Swift | Deno |
| --- | --- | --- | --- | --- | --- | --- |
| Single binary | ✅ | ✅ | ⚠️ pkg | ⚠️ PyInstaller | ✅ | ⚠️ large |
| No runtime needed | ✅ | ✅ | ❌ | ❌ | ✅ | ❌ |
| CLI ecosystem | ⭐⭐⭐ | ⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐ | ⭐⭐ |
| Concurrency | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐ | ⭐⭐ | ⭐⭐ |
| Dev velocity | ⭐ | ⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐⭐ |
| Cross-platform | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐⭐ | ⭐ | ⭐⭐ |
| Performance | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ | ⭐ | ⭐⭐⭐ | ⭐⭐ |
| Binary size | ~3-5MB | ~8-12MB | ~50MB | ~40MB | ~5-10MB | ~80MB |

## Decision

**Rust** — Confirmed as the implementation language.

---

## Recommendation

### **Primary: Rust** ⭐

**Why Rust is the best fit for Gisa:**

1. **Zero-friction distribution**: Users run `brew install gisa` or download a binary. No "install Node first" or "use Python 3.9+". This is critical for CLI adoption.

2. **`clap`**** is exceptional**: Derive macros generate argument parsing, help text, shell completions, and validation from struct definitions. Less code, fewer bugs.

3. **Fearless concurrency**: Parallel cloning is safe by default. Rust's ownership model prevents data races at compile time.

4. **Small, fast binaries**: ~3-5MB binary that starts instantly. Users expect CLI tools to be snappy.

5. **Reliability**: If it compiles, it usually works. The type system catches entire categories of bugs.

6. **Shell out to ****`git`**: Don't fight `libgit2`. Shell out to the user's `git` binary — it's what they expect and handles auth/SSH correctly.

### **Alternative: Go**

If Rust's learning curve is a concern, Go is a solid second choice:

- Simpler language, faster to prototype
- Also produces single binaries
- `go-git` works well for pure-Go git operations
- Goroutines make concurrency straightforward

The tradeoff is more boilerplate and less expressive error handling.

### **Not Recommended for V1**

- **TypeScript/Python**: Runtime requirement is a dealbreaker for CLI distribution
- **Swift**: Limited to macOS, smaller ecosystem
- **Deno**: Immature, large binary sizes

## Recommended Rust Stack

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }      # CLI parsing
tokio = { version = "1", features = ["full"] }       # Async runtime
reqwest = { version = "0.11", features = ["json"] }  # HTTP client
serde = { version = "1", features = ["derive"] }     # JSON serialization
serde_yaml = "0.9"                                   # Config file parsing
indicatif = "0.17"                                   # Progress bars
console = "0.15"                                     # Terminal colors/styling
directories = "5"                                    # XDG paths
thiserror = "1"                                      # Error handling
```

## Example CLI Structure (Rust + Clap)

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gisa")]
#[command(about = "Mirror GitHub org/repo structure locally")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Clone all repos from your GitHub orgs
    Clone {
        /// Base directory for cloned repos
        #[arg(default_value = "~/github")]
        path: String,

        /// Parallel clone operations
        #[arg(short, long, default_value = "4")]
        jobs: usize,

        /// Preview without cloning
        #[arg(long)]
        dry_run: bool,
    },

    /// Sync existing clones with remote
    Sync {
        /// Base directory
        path: String,

        /// Sync mode
        #[arg(long, default_value = "fetch")]
        mode: SyncMode,
    },

    /// Initialize config file
    Init,
}
```

This generates:
- `gisa --help`
- `gisa clone --help`
- Shell completions for bash/zsh/fish
- Typed, validated arguments
