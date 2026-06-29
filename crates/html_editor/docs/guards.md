# Guards
Use `if` when a block should run only when a check is true.

```puzzle
rules {
if count(Box) == 0 {
next_level
}
}
```

The guarded block runs only when the condition is true.

Named conditions can also be used inside `if`.

