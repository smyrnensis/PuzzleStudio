#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteOrderSurface {
    /// Cell-coordinate direction priority, from the most significant
    /// comparison to the least significant comparison. A cell on the named
    /// direction side is drawn in front when that comparison is the first one
    /// that differs.
    pub priority: Vec<String>,
    pub items: Vec<SpriteOrderItemSurface>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpriteOrderItemSurface {
    /// One render priority. Resolution may prove that its members are mutually
    /// exclusive (for example, because they occupy the same state slot).
    Priority(Vec<String>),
    /// One unordered render priority. `A + B` is surface sugar for this node.
    Merge(Vec<String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteOrderSurfaceError {
    InvalidHeader,
    MissingClosingBrace,
    DuplicateDirectionPriority,
    MissingPriorityDirection,
    InvalidPriorityAssignment,
    EmptyPriority,
    EmptyMerge,
    InvalidMergeBlock,
    PlusNeedsOperand,
    MixedPlusAndPlainPriority,
}

impl SpriteOrderSurfaceError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidHeader => "sprite order block must be: order { ... }",
            Self::MissingClosingBrace => "sprite order block is missing its closing brace",
            Self::DuplicateDirectionPriority => {
                "sprite order block may declare direction priority only once"
            }
            Self::MissingPriorityDirection => {
                "sprite order priority requires at least one direction"
            }
            Self::InvalidPriorityAssignment => {
                "sprite order direction priority must be: priority = <direction...>"
            }
            Self::EmptyPriority => "sprite order priority must not be empty",
            Self::EmptyMerge => "sprite merge must contain at least one selector",
            Self::InvalidMergeBlock => "sprite merge must be: merge { <selector...> }",
            Self::PlusNeedsOperand => "sprite merge + requires a selector on both sides",
            Self::MixedPlusAndPlainPriority => {
                "a sprite order row cannot mix + composition with a plain priority list"
            }
        }
    }
}

/// Parses an `order { ... }` block and returns its canonical, dimension-neutral
/// tree together with the first unconsumed line. `+` never survives this step:
/// it is normalized to [`SpriteOrderItemSurface::Merge`].
pub fn parse_sprite_order_surface(
    lines: &[String],
    start: usize,
) -> Result<(SpriteOrderSurface, usize), SpriteOrderSurfaceError> {
    if lines.get(start).map(|line| header_tokens(line)) != Some(vec!["order"]) {
        return Err(SpriteOrderSurfaceError::InvalidHeader);
    }

    let mut priority = None;
    let mut items = Vec::new();
    let mut index = start + 1;
    while let Some(line) = lines.get(index) {
        let trimmed = line.trim();
        if trimmed == "}" {
            return Ok((
                SpriteOrderSurface {
                    priority: priority.unwrap_or_default(),
                    items,
                },
                index + 1,
            ));
        }
        if trimmed.is_empty() {
            index += 1;
            continue;
        }

        if trimmed.starts_with("priority") {
            let directions = parse_priority_assignment(trimmed)?;
            if priority.replace(directions).is_some() {
                return Err(SpriteOrderSurfaceError::DuplicateDirectionPriority);
            }
            index += 1;
            continue;
        }

        if trimmed.starts_with("merge") {
            let (selectors, next) = parse_merge(lines, index)?;
            items.push(SpriteOrderItemSurface::Merge(selectors));
            index = next;
            continue;
        }

        items.push(parse_priority_or_plus(trimmed)?);
        index += 1;
    }

    Err(SpriteOrderSurfaceError::MissingClosingBrace)
}

fn parse_priority_assignment(line: &str) -> Result<Vec<String>, SpriteOrderSurfaceError> {
    let Some((left, right)) = super::parse_assignment_row(line) else {
        return Err(SpriteOrderSurfaceError::InvalidPriorityAssignment);
    };
    if left != "priority" {
        return Err(SpriteOrderSurfaceError::InvalidPriorityAssignment);
    }
    let directions = selectors(right);
    if directions.is_empty() {
        return Err(SpriteOrderSurfaceError::MissingPriorityDirection);
    }
    Ok(directions)
}

fn parse_merge(
    lines: &[String],
    start: usize,
) -> Result<(Vec<String>, usize), SpriteOrderSurfaceError> {
    let line = lines[start].trim();
    let Some(after_keyword) = line.strip_prefix("merge") else {
        return Err(SpriteOrderSurfaceError::InvalidMergeBlock);
    };
    let after_keyword = after_keyword.trim_start();
    let Some(after_open) = after_keyword.strip_prefix('{') else {
        return Err(SpriteOrderSurfaceError::InvalidMergeBlock);
    };

    if let Some((body, trailing)) = after_open.split_once('}') {
        if !trailing.trim().is_empty() {
            return Err(SpriteOrderSurfaceError::InvalidMergeBlock);
        }
        let members = merge_members(body);
        if members.is_empty() {
            return Err(SpriteOrderSurfaceError::EmptyMerge);
        }
        return Ok((members, start + 1));
    }

    if !after_open.trim().is_empty() {
        return Err(SpriteOrderSurfaceError::InvalidMergeBlock);
    }
    let mut members = Vec::new();
    let mut index = start + 1;
    while let Some(line) = lines.get(index) {
        let trimmed = line.trim();
        if trimmed == "}" {
            if members.is_empty() {
                return Err(SpriteOrderSurfaceError::EmptyMerge);
            }
            return Ok((members, index + 1));
        }
        if trimmed.contains('{') || trimmed.contains('}') || trimmed.contains('+') {
            return Err(SpriteOrderSurfaceError::InvalidMergeBlock);
        }
        members.extend(merge_members(trimmed));
        index += 1;
    }
    Err(SpriteOrderSurfaceError::MissingClosingBrace)
}

fn parse_priority_or_plus(line: &str) -> Result<SpriteOrderItemSurface, SpriteOrderSurfaceError> {
    if !line.contains('+') {
        let values = selectors(line);
        if values.is_empty() {
            return Err(SpriteOrderSurfaceError::EmptyPriority);
        }
        return Ok(SpriteOrderItemSurface::Priority(values));
    }

    let mut members = Vec::new();
    for operand in line.split('+') {
        let values = selectors(operand);
        if values.is_empty() {
            return Err(SpriteOrderSurfaceError::PlusNeedsOperand);
        }
        if values.len() != 1 {
            return Err(SpriteOrderSurfaceError::MixedPlusAndPlainPriority);
        }
        members.push(values.into_iter().next().expect("one merge operand"));
    }
    Ok(SpriteOrderItemSurface::Merge(members))
}

fn merge_members(body: &str) -> Vec<String> {
    body.split(';').flat_map(selectors).collect()
}

fn selectors(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn header_tokens(line: &str) -> Vec<&str> {
    let mut tokens = line.split_whitespace().collect::<Vec<_>>();
    if tokens.last().copied() == Some("{") {
        tokens.pop();
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plus_is_sugar_for_unordered_merge_node() {
        let plus = vec![
            "order {".to_string(),
            "A + B + Group".to_string(),
            "}".to_string(),
        ];
        let canonical = vec![
            "order {".to_string(),
            "merge { A; B; Group }".to_string(),
            "}".to_string(),
        ];

        let (plus, _) = parse_sprite_order_surface(&plus, 0).unwrap();
        let (canonical, _) = parse_sprite_order_surface(&canonical, 0).unwrap();

        assert_eq!(plus, canonical);
        assert_eq!(
            plus.items,
            vec![SpriteOrderItemSurface::Merge(vec![
                "A".to_string(),
                "B".to_string(),
                "Group".to_string(),
            ])]
        );
    }

    #[test]
    fn direction_priority_preserves_lexicographic_significance() {
        let lines = vec![
            "order {".to_string(),
            "priority = down right front".to_string(),
            "background".to_string(),
            "Actor Shadow".to_string(),
            "}".to_string(),
        ];

        let (surface, next) = parse_sprite_order_surface(&lines, 0).unwrap();

        assert_eq!(next, lines.len());
        assert_eq!(surface.priority, ["down", "right", "front"]);
        assert_eq!(
            surface.items,
            vec![
                SpriteOrderItemSurface::Priority(vec!["background".to_string()]),
                SpriteOrderItemSurface::Priority(vec!["Actor".to_string(), "Shadow".to_string(),]),
            ]
        );
    }

    #[test]
    fn group_alone_is_valid_inside_merge() {
        let lines = vec![
            "order {".to_string(),
            "merge {".to_string(),
            "CrossSlotGroup".to_string(),
            "}".to_string(),
            "}".to_string(),
        ];

        let (surface, _) = parse_sprite_order_surface(&lines, 0).unwrap();

        assert_eq!(
            surface.items,
            vec![SpriteOrderItemSurface::Merge(vec![
                "CrossSlotGroup".to_string()
            ])]
        );
    }
}
