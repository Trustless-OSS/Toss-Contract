# Contributing

Thank you for contributing to Trustless-OSS. Keep changes focused, preserve the existing contract boundaries, and include tests for behavior changes.

## Local workflow

```bash
git checkout main
git pull --ff-only origin main
git checkout -b <branch-name>
```

Before opening a pull request, run:

```bash
cargo fmt --all -- --check
cargo build --workspace --verbose
cargo test --workspace --verbose
```

The GitHub Actions workflow runs the build and test commands for pushes and pull requests targeting `main`.

## Branch naming

Use a lowercase type followed by a short kebab-case description:

```text
<type>/<short-description>
```

Recommended types:

| Type | Use for | Example |
| --- | --- | --- |
| `feat` | New contract behavior | `feat/cctp-payout-validation` |
| `fix` | Bug fixes | `fix/available-balance-check` |
| `docs` | README or documentation changes | `docs/contract-architecture` |
| `test` | Test-only changes | `test/milestone-cancellation` |
| `refactor` | Internal code changes without behavior changes | `refactor/storage-helpers` |
| `ci` | Workflow or automation changes | `ci/add-format-check` |
| `chore` | Maintenance work | `chore/update-soroban-sdk` |

Avoid spaces, vague names such as `changes`, and branch names that combine unrelated work.

## Commit-message format

Use a concise Conventional Commits-style subject:

```text
<type>(<scope>): <imperative summary>
```

Examples:

```text
feat(milestones): add contributor reassignment
fix(storage): extend milestone TTL after writes
test(payouts): cover invalid CCTP recipients
docs(readme): simplify contributor onboarding
ci(rust): run workspace tests on pull requests
```

Keep the subject in the imperative mood, start it with a lowercase word, and aim for 72 characters or fewer. Use the body when the reason or trade-off is not obvious:

```text
fix(balance): prevent withdrawal of reserved funds

Compute available funds after subtracting reserved and released amounts.
This keeps milestone rewards locked while allowing unused funds to move.
```

Use `!` or a `BREAKING CHANGE:` footer only when the public contract interface or behavior changes:

```text
feat(api)!: rename initialize entry point

BREAKING CHANGE: callers must use initialize instead of initialize_escrow.
```

## Pull request checklist

- Explain what changed and why.
- Keep the pull request limited to one coherent change.
- Add or update tests for authorization, balances, state transitions, and events when applicable.
- Confirm `cargo build --workspace` and `cargo test --workspace` pass.
- Mention known limitations or follow-up work.
- Do not include secrets, generated deployment credentials, or unrelated formatting changes.

## Documentation map

- [Architecture](arch.md) — system context, modules, storage, and Mermaid diagrams.
- [Contract specification](contract-spec.md) — entry points, data model, errors, deployment, and limitations.
- [Repository README](../README.md) — installation, build, test, and project overview.
