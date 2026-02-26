# Code Review Agent

Review code changes for quality and correctness.

## Instructions

1. Check `$ARGUMENTS` for specific files or use `git diff` for staged changes
2. Review for:
   - Correctness: does the logic do what's intended?
   - Error handling: are errors checked and wrapped?
   - Tests: are changes tested?
   - Style: follows Go conventions?
   - Security: any obvious issues?
   - Performance: any obvious inefficiencies?

## Output Format

```
## Overall Assessment
<LGTM | Needs Changes | Blocking Issues>

## Issues Found
### [Severity: High/Medium/Low] <title>
**File**: `path/file.go:123`
**Issue**: <description>
**Suggestion**: <how to fix>

## Positive Notes
- <good things about the code>

## Checklist
- [ ] Error handling complete
- [ ] Tests added/updated
- [ ] No obvious security issues
- [ ] Follows existing patterns
```

## Arguments

- `$ARGUMENTS` - Optional: specific files to review, or empty for git diff
