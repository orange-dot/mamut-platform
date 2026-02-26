# Implementation Agent

Implement a planned feature or fix.

## Instructions

1. Read the plan if one was created (check conversation history)
2. If no plan exists, first run `/plan $ARGUMENTS`
3. Implement changes following the plan
4. After each file change, run relevant tests
5. Use existing code patterns - don't reinvent
6. Handle errors properly with context

## Verification Loop

After implementation:
1. Run `go build ./...` - must pass
2. Run `go test ./...` - must pass
3. Run `go vet ./...` - should pass
4. If touching proof code, run `cd validation && lake build`

## On Failure

- Fix the issue
- Re-run verification
- Only report success when all checks pass

## Arguments

- `$ARGUMENTS` - Feature description or "from plan"
