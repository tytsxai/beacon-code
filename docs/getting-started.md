## Getting started

Looking for something specific? Jump ahead:

- [Tips & shortcuts](#tips--shortcuts) – hotkeys, resume flow, prompts
- [Non-interactive runs](./exec.md) – automate with `code exec`
- Ready for deeper customization? Head to [`advanced.md`](./advanced.md)

### CLI usage

Primary command is `beacon`; `code` remains a short alias (and `coder` is the conflict-safe fallback). Examples below use `code` for brevity.

| Command            | Purpose                            | Example                         |
| ------------------ | ---------------------------------- | ------------------------------- |
| `code`            | Interactive TUI                    | `code`                         |
| `code "..."`      | Initial prompt for interactive TUI | `code "fix lint errors"`       |
| `code exec "..."` | Non-interactive "automation mode"  | `code exec "explain utils.ts"` |

Key flags: `--model/-m`, `--ask-for-approval/-a`.

### Resuming interactive sessions

- Run `code resume` to display the session picker UI
- Resume most recent: `code resume --last`
- Resume by id: `code resume <SESSION_ID>` (You can get session ids from /status or `~/.code/sessions/`)
- The picker shows the session's recorded Git branch when available.
- To show the session's original working directory (CWD), run `code resume --all` (this also disables cwd filtering and adds a `CWD` column).

Examples:

```shell
# Open a picker of recent sessions
code resume

# Resume the most recent session
code resume --last

# Resume a specific session by id
code resume 7f9f9a2e-1b3c-4c7a-9b0e-123456789abc
```

### Running with a prompt as input

You can also run Beacon CLI with a prompt as input:

```shell
code "explain this codebase to me"
```

### Example prompts

Below are a few bite-size examples you can copy-paste. Replace the text in quotes with your own task.

| ✨  | What you type                                                                   | What happens                                                               |
| --- | ------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| 1   | `code "Refactor the Dashboard component to React Hooks"`                       | Beacon rewrites the class component, runs `npm test`, and shows the diff.   |
| 2   | `code "Generate SQL migrations for adding a users table"`                      | Infers your ORM, creates migration files, and runs them in a sandboxed DB. |
| 3   | `code "Write unit tests for utils/date.ts"`                                    | Generates tests, executes them, and iterates until they pass.              |
| 4   | `code "Bulk-rename *.jpeg -> *.jpg with git mv"`                               | Safely renames files and updates imports/usages.                           |
| 5   | `code "Explain what this regex does: ^(?=.*[A-Z]).{8,}$"`                      | Outputs a step-by-step human explanation.                                  |
| 6   | `code "Carefully review this repo, and propose 3 high impact well-scoped PRs"` | Suggests impactful PRs in the current codebase.                            |
| 7   | `code "Look for vulnerabilities and create a security review report"`          | Finds and explains security bugs.                                          |

Looking to reuse your own instructions? Create slash commands with [custom prompts](./prompts.md).

### Memory with AGENTS.md

You can give Beacon extra instructions and guidance using `AGENTS.md` files. Beacon looks for them in the following places, and merges them top-down:

1. Global guidance: `~/.code/AGENTS.md` (Beacon Code).
2. Project guidance: every directory from the Git repo root down to your current working directory (inclusive), concatenated in that order.

Notes:

- This repo contains two Rust workspaces with slightly different discovery rules. See `docs/agents_md.md` for the authoritative behavior per workspace.
- `AGENTS.override.md` is supported in `third_party/upstream/codex-rs/`, but is not currently recognized in `code-rs/`.

For more information on how to use AGENTS.md, see the [official AGENTS.md documentation](https://agents.md/).

### Tips & shortcuts

#### Use `@` for file search

Typing `@` triggers a fuzzy-filename search over the workspace root. Use up/down to select among the results and Tab or Enter to replace the `@` with the selected path. You can use Esc to cancel the search.

#### Esc–Esc to edit a previous message

When the chat composer is empty, press Esc to prime “backtrack” mode. Press Esc again to open a transcript preview highlighting the last user message; press Esc repeatedly to step to older user messages. Press Enter to confirm and Beacon will fork the conversation from that point, trim the visible transcript accordingly, and pre‑fill the composer with the selected user message so you can edit and resubmit it.

In the transcript preview, the footer shows an `Esc edit prev` hint while editing is active.

#### `--cd`/`-C` flag

Sometimes it is not convenient to `cd` to the directory you want Beacon to use as the "working root" before running Beacon. Fortunately, `code` supports a `--cd` option so you can specify whatever folder you want. You can confirm that Beacon is honoring `--cd` by double-checking the **workdir** it reports in the TUI at the start of a new session.

#### `--add-dir` flag

Need to work across multiple projects in one run? Pass `--add-dir` one or more times to expose extra directories as writable roots for the current session while keeping the main working directory unchanged. For example:

```shell
code --cd apps/frontend --add-dir ../backend --add-dir ../shared
```

Beacon can then inspect and edit files in each listed directory without leaving the primary workspace.

#### Shell completions

Generate shell completion scripts via:

```shell
code completion bash
code completion zsh
code completion fish
```

#### Image input

Paste images directly into the composer (Ctrl+V / Cmd+V) to attach them to your prompt. You can also attach files via the CLI using `-i/--image` (comma‑separated):

```bash
code -i screenshot.png "Explain this error"
code --image img1.png,img2.jpg "Summarize these diagrams"
```

#### Environment variables and executables

Make sure your environment is already set up before launching Beacon so it does not spend tokens probing what to activate. For example, source your Python virtualenv (or other language runtimes), start any required daemons, and export the env vars you expect to use ahead of time.
