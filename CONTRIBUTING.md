<!-- Target path: nabhold/engine-template/CONTRIBUTING.md (becomes <new-repo>/CONTRIBUTING.md in any repo created from this template). -->

# Contributing

This repository follows the `nabhold` org's standard workflow:

- All changes land through a pull request against `main` — no direct pushes.
- At least one CODEOWNERS-required review is mandatory (see
  `.github/CODEOWNERS`); files shared with `nabhold/shared` contracts may
  require two.
- Required CI status checks (once activated — see `TEMPLATE-USAGE.md`) must
  pass before merge.
- Development happens inside the `baobab-dev` devcontainer declared in
  `.devcontainer/devcontainer.json` — see that file and
  `.nabhold/environment.yaml` for the exact toolchain this repo expects.
- Keep `CHANGELOG.md` current — add one once this repo starts cutting
  releases; it doesn't need one while still scaffolded.

Nothing else is engine-specific yet. Replace or extend this file once this
repo has real build/test/lint commands worth documenting.
