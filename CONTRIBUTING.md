# Contribution Policy

This repository is owner maintained.

1. Author identity for commits must be `chochy2001 <54371626+chochy2001@users.noreply.github.com>`.
2. Code ownership is enforced by `.github/CODEOWNERS` with `@chochy2001`.
3. Automated commit metadata from external assistants is not accepted.
4. Before push, run `make check`, `make test`, `make audit`, and `make verify-authors`.

## Author Verification

Use:

```bash
make verify-authors
```

CI also validates commit author identity on every push and pull request.
