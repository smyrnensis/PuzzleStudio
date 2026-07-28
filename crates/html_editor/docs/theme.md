# Theme
Use `theme` to set presentation colors. Themes expose only `background_color`,
`text_color`, and `accent_color`; derived surfaces should use those colors with
alpha instead of introducing extra theme colors.

```puzzle
theme = clean
theme {
accent_color = #2f7d62
background_color = #f7f5ef
text_color = #1d2522
}
```

Themes set defaults for exported play surfaces.

Use visuals for object drawings and themes for surrounding presentation.
