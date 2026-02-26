# Test Runner

Run tests for the neuro-symbolic framework.

## Instructions

1. Run `go test ./...` to execute all tests
2. If a specific package is mentioned, run `go test ./internal/<package>/...`
3. Report failures clearly with file:line references
4. For Lean validation, run `cd validation && lake build`

## On Failure

- Identify the failing test and root cause
- Suggest a fix but don't implement unless asked
- Check if it's a flaky test or real regression

## Arguments

- `$ARGUMENTS` - Optional: specific package or test name to run
