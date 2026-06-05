# v0.1.6 2026-06-03

## Feature

- Ship native Git subcommands: `git open`, `git ac` (AI commit), `git issues`
- `git ac` commits the staged changes (like `git commit`); `-a` stages all
  changes first, `-e` opens the editor, `-p` previews only

## Change

- Remove the `git-github` umbrella binary; each command is now its own native
  subcommand
- AI commit: bare run commits only the staged changes, `-a` stages all first,
  `-e` opens the editor
- With nothing staged, `git ac` prints a hint to use `git add` or `git ac -a`

## Fix

- `open -b`/`-c` were ignored and always opened the current branch; explicit
  targets are now honored
- Staged diff sent to the AI no longer repeats the full diff once per file
- `open -r <unknown>` and running outside a repo now print a clear error
  instead of panicking
- The auto-generated config file is now valid TOML (the old `prompt` default
  was malformed and broke first run)

## Other

- AI commit now skips binary, lock/generated, and oversized files when building
  the prompt (they are still committed)
- Configurable model via `[deepseek] model`; project-local config renamed to
  `git-github.toml`; cross-platform home-directory lookup

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
