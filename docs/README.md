# Documentation Index

This folder groups project documentation by purpose. The guides are written in Spanish; filenames and directory names use English for consistency.

## Architecture

- [Architecture](architecture/ARCHITECTURE.md): crate structure, extraction flow, and component responsibilities.

## User and format references

- [CLI Reference](reference/CLI.md): commands, arguments, and output behavior.
- [Format Support](reference/FORMATS.md): implemented formats, CLI coverage, and known limits.

## Quality and integrity

- [Integrity Review](quality/INTEGRITY_REVIEW.md): observed defects, validation evidence, and remaining risks.

## Plans

- [Integrity Remediation Plan](planning/INTEGRITY_REMEDIATION_PLAN.md): one-cycle-at-a-time remediation work.
- [Format Support Plan](planning/FORMAT_SUPPORT_PLAN.md): implementation roadmap for additional format support.

## Test data

- `../tests/test_extract/` and `../tests/test_tim/` contain versioned regression fixtures.
- `../test_extract/` is a local ignored folder for generated extraction and conversion outputs. Do not store required test fixtures there.
