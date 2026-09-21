# Ship a manpage per subcommand

## Context

`git-same` ships exactly one manpage. `toolkit/packaging/gen-manpage.sh` runs the
`gen-manpage` helper, redirects stdout into `git-same.1`, and the Homebrew CLI
formula installs that single file (`man1.install "git-same.1"` in
`toolkit/homebrew/formula-cli.rb.tmpl:39`).

The helper renders only the top-level command:

```rust
let man = Man::new(Cli::command());
man.render(&mut io::stdout())
```

`Man::render` does not descend into subcommands. It emits a `SUBCOMMANDS` section
that lists `git-same-monitor(1)`, `git-same-sync(1)` and friends by name, but
those pages have never existed. The result is a 2405-byte page that cross-
references eleven manpages we do not ship, so `man git-same-monitor` fails and
`man git-same` documents no subcommand's flags. Users can only discover
`--start`, `--interval`, `--dry-run` and the rest through `--help`.

This was found while investigating why a test asserting "private monitor flags
stay out of the manpage" passed: the flags it checked belong to the `monitor`
subcommand, which that page never renders. The assertion was vacuous and was
removed in `43609e8`. Generating real per-subcommand pages closes the
documentation gap and makes that assertion worth reinstating.

## Approach

`clap_mangen` already provides the whole feature as a free function:

```rust
pub fn generate_to(cmd: clap::Command, out_dir: impl AsRef<Path>) -> io::Result<()>
```

It recurses through subcommands, skips hidden ones (`!s.is_hide_set()`), calls
`disable_help_subcommand(true)` so no `git-same-help.1` is produced, and names
each file from `get_display_name()`. No manual recursion or naming logic needed.

I verified this against the real CLI. It produces twelve pages, including nested
`workspace` children:

```
git-same.1                     git-same-monitor.1     git-same-sync.1
git-same-init.1                git-same-refresh.1     git-same-workspace.1
git-same-scan.1                git-same-reset.1       git-same-workspace-list.1
git-same-setup.1               git-same-status.1      git-same-workspace-default.1
```

`git-same-monitor.1` renders all seven public options and none of the six
`hide = true` ones, confirming `hide` is honoured at this level.

## Changes

### 1. `crates/git-same-cli/src/bin/gen_manpage.rs`

Switch from "render one page to stdout" to "write a directory of pages". Take the
output directory as `argv[1]`, matching how `gen_completions.rs` is invoked, and
keep the existing `eprintln!` + `process::exit(1)` error style:

```rust
let out_dir = match env::args().nth(1) { ... };  // usage error -> exit 2
fs::create_dir_all(&out_dir)?;
clap_mangen::generate_to(Cli::command(), &out_dir)?;
```

Print each written path so the release log stays readable.

### 2. `toolkit/packaging/gen-manpage.sh`

Drop the `> "$OUT_PATH"` redirect and pass `OUT_DIR` through to the helper. Update
the header comment, which currently promises `OUT_DIR/git-same.1`. Keep the
`$# -ne 1` usage guard and the `==> manpage -> ...` progress line.

This is the one interface change: the helper now writes files instead of
streaming to stdout. Both callers are in this repo, so nothing external breaks.

### 3. `toolkit/homebrew/formula-cli.rb.tmpl`

Replace the single-file install with a glob so new subcommands are picked up
automatically instead of needing a template edit each time:

```ruby
man1.install Dir["git-same*.1"]
```

### 4. `.github/workflows/S2-Release-GitHub.yml`

The "Generate manpage" step at line 321 already passes `staging` and needs no
change. Add an assertion after it that `git-same-monitor.1` exists and that
`staging` holds more than one `.1` file, so a regression that silently drops back
to a single page fails the release rather than shipping.

### 5. Optional cleanup: `crates/git-same-cli/src/cli.rs:140,143`

The `reset` doc comment contains two em dashes, which now render into
`git-same-reset.1`. Replace with a colon and a comma respectively.

## Tests

In `crates/git-same-cli/src/cli_tests.rs`, gated behind `release-tools`:

- Generate into a `tempfile::TempDir` and assert the expected filenames exist,
  including nested `git-same-workspace-default.1`.
- Assert no `git-same-help.1` is written.
- Reinstate the private-flag assertion, now against `git-same-monitor.1`, where
  it is meaningful: none of `--managed`, `--install-agent`, `--remove-agent`,
  `--app-path`, `--installer-copy`, `--agent-protocol-version` appear, while
  `--start`, `--stop`, `--status`, `--uninstall` do. Match the roff-escaped form
  (`\-\-start`), not the plain text.

## Verification

```bash
cargo test -p git-same --all-features
bash toolkit/packaging/gen-manpage.sh /tmp/manstage
man /tmp/manstage/git-same-monitor.1     # renders, lists the public flags
man /tmp/manstage/git-same-workspace-default.1
```

Then `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
--all-features -- -D warnings`, and a dispatched S1 run.

Packaging is only fully exercised by a tagged S2 run; the added staging
assertion is what guards it in the meantime.
