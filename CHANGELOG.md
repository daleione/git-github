# Unreleased

## Feature

- `git pr`: push the current branch and open a GitHub pull request with an
  AI-generated title and description (from the commits/diff vs the base branch);
  supports `-b/--base`, `-d/--draft`, `-e/--edit`, `-r/--remote`
- `git ac`: on a TTY, review the generated message before committing —
  accept / edit / regenerate (with optional guidance) / abort; piped input
  still commits immediately
- `git open <path>`: open a file on GitHub, optionally anchored to a line
  (`path:42`) or range (`path:40-50`), at the current branch/commit
- GitHub token resolution falls back to the `gh` CLI (`gh auth token`), so a
  machine authenticated with `gh auth login` works without exporting a token

## Fix

- `git pr` now pushes the branch only once the PR is about to be created (after
  any editor review), and gains `--no-push` to skip pushing entirely
- `git ac` aborts instead of committing when the review prompt receives EOF
  (Ctrl-D)
- `git issues`/`git pr` no longer panic at startup: use native-tls for reqwest
  so it does not add a second rustls CryptoProvider alongside octocrab's, and
  build the GitHub client inside the tokio runtime so it has a reactor

# v0.1.6 2026-06-03

## Feature

- Ship native Git subcommands: `git open`, `git ac` (AI commit), `git issues`
- `git ac` commits the staged changes (like `git commit`); `-a` stages all
  changes first, `-e` opens the editor, `-p` previews only
- `git ac` now commits via `git commit`, so pre-commit/commit-msg hooks and
  commit signing run (libgit2 would have skipped them)
- `git issues` gains `-s/--state` (open/closed/all) and `-r/--remote`, paginates
  through all results, and omits pull requests
- `DEEPSEEK_API_KEY` env var can supply the API key (overrides the config file)
- `git issues` uses `GITHUB_TOKEN`/`GH_TOKEN` when set, for private repos and a
  higher rate limit

## Change

- Remove the `git-github` umbrella binary; each command is now its own native
  subcommand
- AI commit: bare run commits only the staged changes, `-a` stages all first,
  `-e` opens the editor
- With nothing staged, `git ac` prints a hint to use `git add` or `git ac -a`

## Fix

- AI commit no longer silently drops tokens when an SSE line or multibyte
  character is split across network chunks
- AI commit aborts with a clear error instead of committing an empty message
- AI requests now use connect/read timeouts so a stalled connection fails fast
- `open -b`/`-c` were ignored and always opened the current branch; explicit
  targets are now honored
- Staged diff sent to the AI no longer repeats the full diff once per file
- `open -r <unknown>` and running outside a repo now print a clear error
  instead of panicking
- The auto-generated config file is now valid TOML (the old `prompt` default
  was malformed and broke first run)
- Commit banners are centered by character width, so emoji titles align

## Other

- AI commit now skips binary, lock/generated, and oversized files when building
  the prompt (they are still committed)
- Configurable model via `[deepseek] model`; project-local config renamed to
  `git-github.toml`; cross-platform home-directory lookup
- Upgrade dependencies: git2 0.21, octocrab 0.53, reqwest 0.13, nom 8; drop
  unused `regex` and `url`

# v0.1.5 2025-12-04

## Fix

- Fix `commit -am` flag not opening editor for commit message editing
- Fix `commit -m` flag not properly opening interactive editor session

# v0.1.4 2024-11-27

## Feature

- open current branch by default

## Improve

- better error message

# v0.1.3

## Fix

- github repo name may have dot character
