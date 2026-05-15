# Rust Rebuild Plan

## Objective

Replace the first shallow scaffold with a Rust-first OpenView foundation that can grow into a detailed agent/worker orchestration product.

## Tasks

1. Remove Node implementation.
2. Add Cargo workspace with core, worker, and CLI crates.
3. Write failing Rust contract tests for:
   - worker resources and sandbox policy;
   - agent graph compilation;
   - approval pause/resume;
   - backend-compatible messages.
4. Implement core types and state machine.
5. Add worker manifest helpers.
6. Add CLI catalog/manifest/demo commands.
7. Rewrite README and docs around workers/resources/backend/sandboxing.
8. Add Rust CI.
9. Run fmt, tests, clippy, and public-copy guard.
10. Commit and push only to `rohitg00/openview`.
