# Implementation Planner

Plan implementation before writing code.

## Instructions

1. Understand the requested feature or fix from `$ARGUMENTS`
2. Explore relevant code paths using Glob and Grep
3. Identify files that need modification
4. Consider edge cases and error handling
5. Check for existing patterns in the codebase to follow
6. Write a clear plan with:
   - Files to modify/create
   - Key changes in each file
   - Testing approach
   - Potential risks or concerns

## Output Format

```
## Summary
<one sentence description>

## Files to Modify
- `path/to/file.go` - <what changes>

## Implementation Steps
1. <step>
2. <step>

## Testing Plan
- <how to verify>

## Risks
- <any concerns>
```

## Arguments

- `$ARGUMENTS` - Feature or fix description
