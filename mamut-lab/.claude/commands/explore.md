# Codebase Explorer

Explore and understand the codebase.

## Instructions

1. Parse the question/topic from `$ARGUMENTS`
2. Use Glob to find relevant files
3. Use Grep to search for patterns
4. Read key files to understand implementation
5. Trace code paths as needed
6. Summarize findings clearly

## Output Format

```
## Summary
<brief answer>

## Relevant Files
- `path/file.go:123` - <what it does>

## Code Flow
1. <step in the flow>
2. <step>

## Key Details
- <important implementation detail>
```

## Common Explorations

- "How does X work?" - trace the code path
- "Where is X defined?" - find the definition
- "What calls X?" - find usages
- "What does package X do?" - summarize package

## Arguments

- `$ARGUMENTS` - Question or topic to explore
