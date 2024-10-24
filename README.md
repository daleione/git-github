# git-github-open

`git-github-open` is a convenient Git command plugin that allows you to quickly open the remote repository URL in your browser. It supports both GitHub and GitLab, making it easier for developers to access remote repositories, specific branches, or commit records for code review, changes, or sharing.

## Features
- **Multi-Platform Support**: Opens remote repository URLs for GitHub and GitLab.
- **Quick Branch Access**: Specify a branch to open.
- **View Specific Commits**: Open a specific commit page directly.
- **Custom Remote Repository**: Specify different remote names (default is `origin`).

## Installation

```bash
# Install using `cargo`
cargo install git-github
```

## Usage

### Open the default remote repository page
Run the following command in the root directory of your repository to open the main page of the remote repository in your browser:

```bash
git github open
```

### Open a specific branch page
Use the `-b` or `--branch` option to open a specific branch. For example, to open the `develop` branch:

```bash
git github open --branch develop
```

### Open a specific commit page
Use the `-c` or `--commit` option to open a specific commit page. For example, to open a commit with ID `abc1234`:

```bash
git github open --commit abc1234
```

### Specify a remote repository name
If the remote repository name is not `origin`, you can use the `-r` or `--remote` option to specify it. For example:

```bash
git github open --remote upstream
```

## Full Options List

```
Usage: git-github open [OPTIONS]

Options:
  -c, --commit <COMMIT>    Open a specific commit page
  -b, --branch <BRANCH>    Open a specific branch page
  -r, --remote <REMOTE>    Specify the remote repository name (default: origin)
  -h, --help               Print help
```

## Examples

1. **Open the repository homepage**
   ```bash
   git github open
   ```
   This will open the homepage of the `origin` remote repository by default.

2. **Open the `develop` branch**
   ```bash
   git github open --branch develop
   ```
   This will open the `develop` branch of the `origin` remote repository.

3. **View a specific commit**
   ```bash
   git github open --commit abc1234
   ```
   This will open the specific commit page for commit ID `abc1234`.

4. **Specify a remote name**
   ```bash
   git github open --remote upstream
   ```
   This will open the homepage of the `upstream` remote repository.

## Contributing
We welcome contributions via Issues or Pull Requests! If you have any questions or suggestions, feel free to reach out.

## License
This project is licensed under the MIT License.
