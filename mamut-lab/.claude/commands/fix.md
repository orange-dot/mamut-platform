# Bug Fix Agent

Fix a reported bug or issue.

## Instructions

1. Understand the bug from `$ARGUMENTS`
2. Reproduce or locate the issue
3. Identify root cause (don't just fix symptoms)
4. Plan the fix
5. Implement minimally - don't refactor unrelated code
6. Add/update tests to prevent regression
7. Verify the fix

## Debugging Steps

1. Find relevant code using Grep/Glob
2. Read the code to understand current behavior
3. Trace the bug to its source
4. Identify the minimal fix

## Verification

After fixing:
1. `go build ./...` - must pass
2. `go test ./...` - must pass
3. Manually verify the specific bug is fixed

## Output

```
## Bug Analysis
**Location**: `file.go:123`
**Root Cause**: <why it happens>

## Fix Applied
**Files Changed**:
- `file.go` - <what changed>

## Verification
- Build: PASS
- Tests: PASS
- Bug fixed: YES
```

## Arguments

- `$ARGUMENTS` - Bug description or error message
