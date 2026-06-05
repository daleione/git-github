# Git-GitHub CLI Tool

A command-line tool for interacting with Git/GitHub repositories: quickly open
repo pages, list issues, and create AI-powered commits.

## Features

- **Open repo pages** directly from your terminal (homepage, branch, or commit)
- **Issue listing** for the current repository
- **AI-powered commits** with automatically generated messages

## Installation

```bash
cargo install git-github          # from crates.io
# or, from a local checkout:
cargo install --path .
```

This installs several binaries into `~/.cargo/bin`. As long as that directory is
on your `PATH`, Git will pick them up as native subcommands.

## Commands

Each command is a native Git subcommand.

| Command      | Description                          |
| ------------ | ------------------------------------ |
| `git open`   | Open the repo page in your browser   |
| `git ac`     | AI-commit the staged changes         |
| `git pr`     | Open a PR with an AI description      |
| `git issues` | List repository issues               |

> `git <cmd> --help` is intercepted by Git to look for a man page. Use the short
> flag `git <cmd> -h` (or call the binary directly, e.g. `git-ac --help`) to see
> the tool's own help.

### `git open`

Open the repository on GitHub.

```bash
git open                 # current branch (or repo homepage when detached)
git open -b dev          # the dev branch
git open -c abc123       # commit abc123
git open -r upstream     # use the 'upstream' remote (default: origin)
```

Options:

- `-c`, `--commit <COMMIT>`: open a specific commit (conflicts with `--branch`)
- `-b`, `--branch <BRANCH>`: open a specific branch
- `-r`, `--remote <REMOTE>`: remote name (default: `origin`)

### `git ac` — AI commit

Generates a commit message from your **staged** changes using the DeepSeek API.
Like `git commit`, it commits only what you have staged; pass `-a` to stage all
changes first.

On an interactive terminal, `git ac` shows the generated message and asks what
to do — **[Y]es** to commit, **[e]dit** in your editor, **[r]egenerate** (with
optional guidance), or **[a]bort**. When the input is piped, it commits straight
away so scripts are unaffected.

```bash
git ac          # commit the staged changes with an AI-generated message
git ac -a       # stage all changes first, then generate and commit
git ac -e       # generate, then open the editor to review before committing
git ac -p       # preview the message only (no staging, no commit)
```

Options:

- `-a`, `--all`: stage all changes before committing (like `git add -A`)
- `-e`, `--edit`: open the editor to review/edit before committing
- `-p`, `--preview`: only preview the message; do not stage or commit

### `git pr` — AI pull request

Pushes the current branch and opens a GitHub pull request with a title and
description generated from the commits and diff against the base branch.

```bash
git pr               # PR into the repo's default branch
git pr -b dev        # target the 'dev' branch
git pr -d            # create as a draft
git pr -e            # edit the title/body before creating
```

Options:

- `-b`, `--base <BASE>`: base branch to merge into (default: the repo's default branch)
- `-d`, `--draft`: create the pull request as a draft
- `-e`, `--edit`: open the editor to review/edit the title and body first
- `-r`, `--remote <REMOTE>`: remote name (default: `origin`)

Requires a `GITHUB_TOKEN` (or `GH_TOKEN`) env var, and a DeepSeek API key for the
description.

### `git issues`

```bash
git issues              # list the current repo's open issues
git issues -s closed    # closed issues (open | closed | all)
git issues -s all
git issues -r upstream  # use the 'upstream' remote (default: origin)
```

Options:

- `-s`, `--state <STATE>`: which issues to list — `open` (default), `closed`, `all`
- `-r`, `--remote <REMOTE>`: remote name (default: `origin`)

Results are paginated through fully, and pull requests are omitted. Set a
`GITHUB_TOKEN` (or `GH_TOKEN`) env var to access private repos and avoid the
unauthenticated rate limit.

## Configuration

On first run a config file is created at
`~/.config/git-github/config.toml` (a `git-github.toml` in the current
directory takes precedence, for per-project overrides). Set your DeepSeek API
key there:

```toml
[deepseek]
api_key = "sk-..."
model = "deepseek-chat"   # any DeepSeek chat model
temperature = 0.7
# Optional: override the default system prompt
prompt = ""
```

Alternatively, set the `DEEPSEEK_API_KEY` env var; it overrides the config file,
so the key never has to be written to disk.

## Contributing

Pull requests are welcome! For major changes, please open an issue first to
discuss what you'd like to change.

## License

[MIT](https://choosealicense.com/licenses/mit/)
