# Git-GitHub CLI Tool

A command-line tool for interacting with Git repositories, featuring quick access to repo pages, issue management, and AI-powered commits.

## Features

- **Open repo pages** directly from your terminal
  - Repository homepage
  - Specific branches
  - Specific commits
- **Issue management**
  - Focus on specific issues
  - List all repository issues
- **AI-powered commits** with automatically generated messages

## Installation

```bash
cargo install git-github
```

## Usage

```bash
git-github [OPTIONS] <COMMAND>
```

### Options

- `-d`, `--debug`: Turn debugging information on (can be used multiple times to increase verbosity)

### Commands

#### Open

Open the repo website in your browser:

```bash
git-github open [OPTIONS]
```

Options:
- `-c`, `--commit <COMMIT>`: Open a specific commit (conflicts with branch)
- `-b`, `--branch <BRANCH>`: Open a specific branch
- `-r`, `--remote <REMOTE>`: Specify remote name (default: "origin")

Examples:
```bash
git-github open                     # Opens the repo homepage
git-github open -b main             # Opens the main branch
git-github open -c abc123           # Opens commit abc123
git-github open -r upstream -b dev  # Opens dev branch on upstream remote
```

#### Issue

Manage GitHub issues:

```bash
git-github issue <COMMAND>
```

Subcommands:
- `focus`: Focus on a specific issue
  ```bash
  git-github issue focus -i <ISSUE_ID>
  ```
- `list`: List all issues
  ```bash
  git-github issue list
  ```

#### Commit

Create an AI-generated commit:

```bash
git-github commit [OPTIONS]
```

Options:
- `-a`, `--apply`: Apply the AI-generated message to the new commit (default: false)

Examples:
```bash
git-github commit       # Shows AI-generated message without committing
git-github commit -a    # Creates commit with AI-generated message
```

## Contributing

Pull requests are welcome! For major changes, please open an issue first to discuss what you'd like to change.

## License

[MIT](https://choosealicense.com/licenses/mit/)
