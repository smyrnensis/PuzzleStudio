use puzzle_grid3d::{
    Delta3, Direction3, DirectionSet3, Frame3, FrameSet3, GapTerm3, Guard3, InputId, LayerId,
    MarkId3, MarkPattern3, MatchCell3, ObjectId, ObjectSetMarkPattern3, Offset3, Pattern3, Rule3,
    RuleApplication3, RuleId3, WriteOp3,
};

pub type SelectorMark3 = puzzle_authoring::SelectorMark;
pub type ObjectSelector3 = puzzle_authoring::ObjectSelector<SelectorMark3>;
pub type SelectorTag3 = puzzle_authoring::SelectorTag;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedSelectorMark3 {
    pub id: MarkId3,
    pub value: Option<i64>,
    pub match_value: puzzle_kernel::MarkValueMatch,
    pub negated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedObjectSelector3 {
    pub token: String,
    pub alternatives: Vec<ObjectId>,
    pub mark: Vec<ResolvedSelectorMark3>,
    pub occurrence_labeled: bool,
    pub runtime_object_set_layer: Option<LayerId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternTemplate3<Selector = ObjectSelector3, Mark = SelectorMark3> {
    pub cells: Vec<MatchCellTemplate3<Selector, Mark>>,
    pub gap_count: u16,
}

impl<Selector, Mark> PatternTemplate3<Selector, Mark> {
    pub fn new(cells: Vec<MatchCellTemplate3<Selector, Mark>>) -> Self {
        Self {
            cells,
            gap_count: 0,
        }
    }

    pub fn with_gap_count(mut self, gap_count: u16) -> Self {
        self.gap_count = gap_count;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchCellTemplate3<Selector = ObjectSelector3, Mark = SelectorMark3> {
    pub offset: Offset3,
    pub require_null: bool,
    pub require: Vec<Selector>,
    pub forbid: Vec<Selector>,
    pub require_cell_mark: Vec<Mark>,
    pub forbid_cell_mark: Vec<Mark>,
}

impl<Selector, Mark> MatchCellTemplate3<Selector, Mark> {
    pub fn new(offset: impl Into<Offset3>) -> Self {
        Self {
            offset: offset.into(),
            require_null: false,
            require: Vec::new(),
            forbid: Vec::new(),
            require_cell_mark: Vec::new(),
            forbid_cell_mark: Vec::new(),
        }
    }

    pub fn require(mut self, selector: Selector) -> Self {
        self.require.push(selector);
        self
    }

    pub fn forbid(mut self, selector: Selector) -> Self {
        self.forbid.push(selector);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DensePattern3 {
    pub slices: Vec<DenseSlice3>,
}

impl DensePattern3 {
    pub fn new(slices: Vec<DenseSlice3>) -> Self {
        Self { slices }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseSlice3 {
    pub rows: Vec<DenseRow3>,
}

impl DenseSlice3 {
    pub fn new(rows: Vec<DenseRow3>) -> Self {
        Self { rows }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseRow3 {
    pub cells: Vec<DenseCell3>,
}

impl DenseRow3 {
    pub fn new(cells: Vec<DenseCell3>) -> Self {
        Self { cells }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseCell3 {
    pub require_null: bool,
    pub require: Vec<ObjectSelector3>,
    pub forbid: Vec<ObjectSelector3>,
    pub require_cell_mark: Vec<SelectorMark3>,
    pub forbid_cell_mark: Vec<SelectorMark3>,
}

impl DenseCell3 {
    pub fn empty() -> Self {
        Self {
            require_null: false,
            require: Vec::new(),
            forbid: Vec::new(),
            require_cell_mark: Vec::new(),
            forbid_cell_mark: Vec::new(),
        }
    }

    pub fn require(selector: ObjectSelector3) -> Self {
        Self::empty().with_required(selector)
    }

    pub fn forbid(selector: ObjectSelector3) -> Self {
        Self::empty().with_forbidden(selector)
    }

    pub fn with_required(mut self, selector: ObjectSelector3) -> Self {
        self.require.push(selector);
        self
    }

    pub fn with_forbidden(mut self, selector: ObjectSelector3) -> Self {
        self.forbid.push(selector);
        self
    }

    fn is_empty(&self) -> bool {
        !self.require_null
            && self.require.is_empty()
            && self.forbid.is_empty()
            && self.require_cell_mark.is_empty()
            && self.forbid_cell_mark.is_empty()
    }
}

pub fn lower_dense_pattern(frame: Frame3, dense: &DensePattern3) -> PatternTemplate3 {
    let mut cells = Vec::new();
    for (depth, slice) in dense.slices.iter().enumerate() {
        for (row, dense_row) in slice.rows.iter().enumerate() {
            for (column, dense_cell) in dense_row.cells.iter().enumerate() {
                if dense_cell.is_empty() {
                    continue;
                }
                let local = Delta3::new(column as i16, row as i16, depth as i16);
                cells.push(MatchCellTemplate3 {
                    offset: frame.to_world_offset(local).into(),
                    require_null: dense_cell.require_null,
                    require: dense_cell.require.clone(),
                    forbid: dense_cell.forbid.clone(),
                    require_cell_mark: dense_cell.require_cell_mark.clone(),
                    forbid_cell_mark: dense_cell.forbid_cell_mark.clone(),
                });
            }
        }
    }
    PatternTemplate3::new(cells)
}

pub fn lower_dense_pattern_set(frames: FrameSet3, dense: &DensePattern3) -> Vec<PatternTemplate3> {
    frames
        .frames()
        .into_iter()
        .map(|frame| lower_dense_pattern(frame, dense))
        .collect()
}

pub fn lower_pattern_template(
    template: &PatternTemplate3<ResolvedObjectSelector3, ResolvedSelectorMark3>,
) -> Result<Vec<Pattern3>, PatternLoweringError3> {
    Ok(lower_pattern_template_with_assignments(template)?
        .into_iter()
        .map(|partial| Pattern3::new(partial.cells))
        .collect())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleTemplate3<Selector = ObjectSelector3, Mark = SelectorMark3> {
    pub id: RuleId3,
    pub guards: Vec<Guard3>,
    pub application: RuleApplication3,
    pub pattern: PatternTemplate3<Selector, Mark>,
    pub writes: Vec<WriteOpTemplate3<Selector, Mark>>,
}

impl<Selector, Mark> RuleTemplate3<Selector, Mark> {
    pub fn once(
        pattern: PatternTemplate3<Selector, Mark>,
        writes: Vec<WriteOpTemplate3<Selector, Mark>>,
    ) -> Self {
        Self {
            id: RuleId3(0),
            guards: Vec::new(),
            application: RuleApplication3::Once,
            pattern,
            writes,
        }
    }

    pub fn once_all(
        pattern: PatternTemplate3<Selector, Mark>,
        writes: Vec<WriteOpTemplate3<Selector, Mark>>,
    ) -> Self {
        Self {
            id: RuleId3(0),
            guards: Vec::new(),
            application: RuleApplication3::OnceAll,
            pattern,
            writes,
        }
    }

    pub fn once_per_level(
        pattern: PatternTemplate3<Selector, Mark>,
        writes: Vec<WriteOpTemplate3<Selector, Mark>>,
    ) -> Self {
        Self {
            id: RuleId3(0),
            guards: Vec::new(),
            application: RuleApplication3::OncePerLevel,
            pattern,
            writes,
        }
    }

    pub fn repeated(
        pattern: PatternTemplate3<Selector, Mark>,
        writes: Vec<WriteOpTemplate3<Selector, Mark>>,
    ) -> Self {
        Self {
            id: RuleId3(0),
            guards: Vec::new(),
            application: RuleApplication3::UntilStable,
            pattern,
            writes,
        }
    }

    pub fn with_id(mut self, id: RuleId3) -> Self {
        self.id = id;
        self
    }

    pub fn when_input(mut self, input: InputId) -> Self {
        self.guards.push(Guard3::InputIs(input));
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WriteOpTemplate3<Selector = ObjectSelector3, Mark = SelectorMark3> {
    Add {
        offset: Offset3,
        object: Selector,
    },
    Remove {
        offset: Offset3,
        object: Selector,
    },
    Replace {
        offset: Offset3,
        remove: Selector,
        add: Selector,
    },
    Move {
        from_offset: Offset3,
        to_offset: Offset3,
        object: Selector,
    },
    SetMark {
        offset: Offset3,
        object: Selector,
        mark: Mark,
    },
    RemoveMark {
        offset: Offset3,
        object: Selector,
        mark: Mark,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineOrientation3 {
    Direction(Direction3),
    DirectionSet(DirectionSet3),
}

impl LineOrientation3 {
    fn directions(&self) -> Vec<Direction3> {
        match self {
            Self::Direction(direction) => vec![*direction],
            Self::DirectionSet(set) => set.directions(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineRuleTemplate3 {
    pub id: RuleId3,
    pub guards: Vec<Guard3>,
    pub application: RuleApplication3,
    pub orientation: LineOrientation3,
    pub pattern: LinePatternTemplate3,
    pub writes: Vec<LineWriteOpTemplate3>,
}

impl LineRuleTemplate3 {
    pub fn once(
        orientation: LineOrientation3,
        pattern: LinePatternTemplate3,
        writes: Vec<LineWriteOpTemplate3>,
    ) -> Self {
        Self {
            id: RuleId3(0),
            guards: Vec::new(),
            application: RuleApplication3::Once,
            orientation,
            pattern,
            writes,
        }
    }

    pub fn with_id(mut self, id: RuleId3) -> Self {
        self.id = id;
        self
    }

    pub fn when_input(mut self, input: InputId) -> Self {
        self.guards.push(Guard3::InputIs(input));
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinePatternTemplate3 {
    pub cells: Vec<LineMatchCellTemplate3>,
    pub gap_count: u16,
}

impl LinePatternTemplate3 {
    pub fn new(cells: Vec<LineMatchCellTemplate3>) -> Self {
        Self {
            cells,
            gap_count: 0,
        }
    }

    pub fn with_gap_count(mut self, gap_count: u16) -> Self {
        self.gap_count = gap_count;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineMatchCellTemplate3 {
    pub step: LineOffsetTemplate3,
    pub require_null: bool,
    pub require: Vec<ObjectSelector3>,
    pub forbid: Vec<ObjectSelector3>,
    pub require_cell_mark: Vec<SelectorMark3>,
    pub forbid_cell_mark: Vec<SelectorMark3>,
}

impl LineMatchCellTemplate3 {
    pub fn new(step: impl Into<LineOffsetTemplate3>) -> Self {
        Self {
            step: step.into(),
            require_null: false,
            require: Vec::new(),
            forbid: Vec::new(),
            require_cell_mark: Vec::new(),
            forbid_cell_mark: Vec::new(),
        }
    }

    pub fn require(mut self, selector: ObjectSelector3) -> Self {
        self.require.push(selector);
        self
    }

    pub fn forbid(mut self, selector: ObjectSelector3) -> Self {
        self.forbid.push(selector);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineOffsetTemplate3 {
    pub base: i16,
    pub gap_terms: Vec<u16>,
}

impl From<i16> for LineOffsetTemplate3 {
    fn from(base: i16) -> Self {
        Self {
            base,
            gap_terms: Vec::new(),
        }
    }
}

impl LineOffsetTemplate3 {
    fn project(&self, direction: Direction3) -> Offset3 {
        if self.gap_terms.is_empty() {
            return direction.offset.scale(self.base).into();
        }
        Offset3::Variable {
            base_dx: direction.offset.dx * self.base,
            base_dy: direction.offset.dy * self.base,
            base_dz: direction.offset.dz * self.base,
            gap_terms: self
                .gap_terms
                .iter()
                .map(|gap_index| GapTerm3 {
                    gap_index: *gap_index,
                    dx: direction.offset.dx,
                    dy: direction.offset.dy,
                    dz: direction.offset.dz,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineWriteOpTemplate3 {
    Add {
        step: LineOffsetTemplate3,
        object: ObjectSelector3,
    },
    Remove {
        step: LineOffsetTemplate3,
        object: ObjectSelector3,
    },
    Replace {
        step: LineOffsetTemplate3,
        remove: ObjectSelector3,
        add: ObjectSelector3,
    },
    Move {
        from_step: LineOffsetTemplate3,
        to_step: LineOffsetTemplate3,
        object: ObjectSelector3,
    },
    SetMark {
        step: LineOffsetTemplate3,
        object: ObjectSelector3,
        mark: SelectorMark3,
    },
    RemoveMark {
        step: LineOffsetTemplate3,
        object: ObjectSelector3,
        mark: SelectorMark3,
    },
}

pub fn project_line_rule_template(template: &LineRuleTemplate3) -> Vec<RuleTemplate3> {
    let mut rules = Vec::new();
    for direction in template.orientation.directions() {
        let pattern = PatternTemplate3::new(
            template
                .pattern
                .cells
                .iter()
                .map(|cell| MatchCellTemplate3 {
                    offset: cell.step.project(direction),
                    require_null: cell.require_null,
                    require: cell
                        .require
                        .iter()
                        .map(|selector| resolve_directional_object_selector3(selector, direction))
                        .collect(),
                    forbid: cell
                        .forbid
                        .iter()
                        .map(|selector| resolve_directional_object_selector3(selector, direction))
                        .collect(),
                    require_cell_mark: cell
                        .require_cell_mark
                        .iter()
                        .map(|mark| resolve_directional_selector_mark(mark, direction))
                        .collect(),
                    forbid_cell_mark: cell
                        .forbid_cell_mark
                        .iter()
                        .map(|mark| resolve_directional_selector_mark(mark, direction))
                        .collect(),
                })
                .collect(),
        )
        .with_gap_count(template.pattern.gap_count);
        let writes = template
            .writes
            .iter()
            .map(|write| line_write_to_world(direction, write))
            .collect();
        rules.push(RuleTemplate3 {
            id: template.id,
            guards: template.guards.clone(),
            application: template.application,
            pattern,
            writes,
        });
    }
    rules
}

fn line_write_to_world(direction: Direction3, write: &LineWriteOpTemplate3) -> WriteOpTemplate3 {
    match write {
        LineWriteOpTemplate3::Add { step, object } => WriteOpTemplate3::Add {
            offset: step.project(direction),
            object: resolve_directional_object_selector3(object, direction),
        },
        LineWriteOpTemplate3::Remove { step, object } => WriteOpTemplate3::Remove {
            offset: step.project(direction),
            object: resolve_directional_object_selector3(object, direction),
        },
        LineWriteOpTemplate3::Replace { step, remove, add } => WriteOpTemplate3::Replace {
            offset: step.project(direction),
            remove: resolve_directional_object_selector3(remove, direction),
            add: resolve_directional_object_selector3(add, direction),
        },
        LineWriteOpTemplate3::Move {
            from_step,
            to_step,
            object,
        } => WriteOpTemplate3::Move {
            from_offset: from_step.project(direction),
            to_offset: to_step.project(direction),
            object: resolve_directional_object_selector3(object, direction),
        },
        LineWriteOpTemplate3::SetMark { step, object, mark } => WriteOpTemplate3::SetMark {
            offset: step.project(direction),
            object: resolve_directional_object_selector3(object, direction),
            mark: resolve_directional_selector_mark(mark, direction),
        },
        LineWriteOpTemplate3::RemoveMark { step, object, mark } => WriteOpTemplate3::RemoveMark {
            offset: step.project(direction),
            object: resolve_directional_object_selector3(object, direction),
            mark: resolve_directional_selector_mark(mark, direction),
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FrameOrientation3 {
    Frame(Frame3),
    FrameSet(FrameSet3),
    Frames(Vec<Frame3>),
}

impl FrameOrientation3 {
    fn frames(&self) -> Vec<Frame3> {
        match self {
            Self::Frame(frame) => vec![*frame],
            Self::FrameSet(set) => set.frames(),
            Self::Frames(frames) => frames.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DenseRuleTemplate3 {
    pub id: RuleId3,
    pub guards: Vec<Guard3>,
    pub application: RuleApplication3,
    pub orientation: FrameOrientation3,
    pub pattern: DensePattern3,
    pub writes: Vec<LocalWriteOpTemplate3>,
}

impl DenseRuleTemplate3 {
    pub fn once(
        orientation: FrameOrientation3,
        pattern: DensePattern3,
        writes: Vec<LocalWriteOpTemplate3>,
    ) -> Self {
        Self {
            id: RuleId3(0),
            guards: Vec::new(),
            application: RuleApplication3::Once,
            orientation,
            pattern,
            writes,
        }
    }

    pub fn with_id(mut self, id: RuleId3) -> Self {
        self.id = id;
        self
    }

    pub fn when_input(mut self, input: InputId) -> Self {
        self.guards.push(Guard3::InputIs(input));
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LocalWriteOpTemplate3 {
    Add {
        offset: Delta3,
        object: ObjectSelector3,
    },
    Remove {
        offset: Delta3,
        object: ObjectSelector3,
    },
    Replace {
        offset: Delta3,
        remove: ObjectSelector3,
        add: ObjectSelector3,
    },
    Move {
        from_offset: Delta3,
        to_offset: Delta3,
        object: ObjectSelector3,
    },
    SetMark {
        offset: Delta3,
        object: ObjectSelector3,
        mark: SelectorMark3,
    },
    RemoveMark {
        offset: Delta3,
        object: ObjectSelector3,
        mark: SelectorMark3,
    },
}

pub fn project_dense_rule_template(template: &DenseRuleTemplate3) -> Vec<RuleTemplate3> {
    let mut rules = Vec::new();
    for frame in template.orientation.frames() {
        let pattern = lower_dense_pattern(frame, &template.pattern);
        let writes = template
            .writes
            .iter()
            .map(|write| local_write_to_world(frame, write))
            .collect();
        rules.push(RuleTemplate3 {
            id: template.id,
            guards: template.guards.clone(),
            application: template.application,
            pattern,
            writes,
        });
    }
    rules
}

fn local_write_to_world(frame: Frame3, write: &LocalWriteOpTemplate3) -> WriteOpTemplate3 {
    match write {
        LocalWriteOpTemplate3::Add { offset, object } => WriteOpTemplate3::Add {
            offset: frame.to_world_offset(*offset).into(),
            object: object.clone(),
        },
        LocalWriteOpTemplate3::Remove { offset, object } => WriteOpTemplate3::Remove {
            offset: frame.to_world_offset(*offset).into(),
            object: object.clone(),
        },
        LocalWriteOpTemplate3::Replace {
            offset,
            remove,
            add,
        } => WriteOpTemplate3::Replace {
            offset: frame.to_world_offset(*offset).into(),
            remove: remove.clone(),
            add: add.clone(),
        },
        LocalWriteOpTemplate3::Move {
            from_offset,
            to_offset,
            object,
        } => WriteOpTemplate3::Move {
            from_offset: frame.to_world_offset(*from_offset).into(),
            to_offset: frame.to_world_offset(*to_offset).into(),
            object: object.clone(),
        },
        LocalWriteOpTemplate3::SetMark {
            offset,
            object,
            mark,
        } => WriteOpTemplate3::SetMark {
            offset: frame.to_world_offset(*offset).into(),
            object: object.clone(),
            mark: mark.clone(),
        },
        LocalWriteOpTemplate3::RemoveMark {
            offset,
            object,
            mark,
        } => WriteOpTemplate3::RemoveMark {
            offset: frame.to_world_offset(*offset).into(),
            object: object.clone(),
            mark: mark.clone(),
        },
    }
}

pub fn lower_rule_template(
    template: &RuleTemplate3<ResolvedObjectSelector3, ResolvedSelectorMark3>,
) -> Result<Vec<Rule3>, RuleLoweringError3> {
    let pattern_partials = lower_pattern_template_with_assignments(&template.pattern)?;
    pattern_partials
        .into_iter()
        .map(|partial| {
            let writes = lower_write_templates(&partial.assignments, &template.writes)?;
            let mut component = puzzle_grid3d::PatternComponent3::new(partial.cells);
            component.gap_count = template.pattern.gap_count;
            Ok(Rule3 {
                id: template.id,
                guards: template.guards.clone(),
                application: template.application,
                pattern: Pattern3::from_components(vec![component]),
                writes,
                effects: Vec::new(),
            })
        })
        .collect()
}

fn lower_pattern_template_with_assignments(
    template: &PatternTemplate3<ResolvedObjectSelector3, ResolvedSelectorMark3>,
) -> Result<Vec<PatternPartial3>, PatternLoweringError3> {
    reject_duplicate_labeled_selectors(template)?;
    let has_labeled_selectors = pattern_template_has_labeled_selectors(template);
    let mut partials = vec![PatternPartial3 {
        cells: Vec::new(),
        assignments: Vec::new(),
    }];

    for cell in &template.cells {
        let mut next_partials = Vec::new();
        for partial in partials {
            next_partials.extend(lower_match_cell_template(cell, partial)?);
        }
        partials = next_partials;
    }

    if has_labeled_selectors {
        partials.reverse();
    }
    Ok(partials)
}

fn pattern_template_has_labeled_selectors(
    template: &PatternTemplate3<ResolvedObjectSelector3, ResolvedSelectorMark3>,
) -> bool {
    template.cells.iter().any(|cell| {
        cell.require
            .iter()
            .any(|selector| selector.occurrence_labeled)
    })
}

fn reject_duplicate_labeled_selectors(
    template: &PatternTemplate3<ResolvedObjectSelector3, ResolvedSelectorMark3>,
) -> Result<(), PatternLoweringError3> {
    let mut seen = Vec::<String>::new();
    for cell in &template.cells {
        for selector in &cell.require {
            if !selector.occurrence_labeled {
                continue;
            }
            let token = selector.token.clone();
            if seen.contains(&token) {
                return Err(PatternLoweringError3::DuplicateSelectorOccurrenceLabel { token });
            }
            seen.push(token);
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PatternPartial3 {
    cells: Vec<MatchCell3>,
    assignments: Vec<SelectorAssignment3>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SelectorAssignment3 {
    token: String,
    value: SelectorAssignmentValue3,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SelectorAssignmentValue3 {
    Object(ObjectId),
    ObjectSet {
        binding: u16,
        layer: LayerId,
        objects: Vec<ObjectId>,
    },
}

fn lower_match_cell_template(
    template: &MatchCellTemplate3<ResolvedObjectSelector3, ResolvedSelectorMark3>,
    partial: PatternPartial3,
) -> Result<Vec<PatternPartial3>, PatternLoweringError3> {
    let PatternPartial3 {
        cells: existing_cells,
        assignments,
    } = partial;
    let mut cell = MatchCell3::new(template.offset.clone());
    cell.require_null = template.require_null;
    apply_selector_mark_to_cell(
        &mut cell,
        ObjectId::EMPTY,
        None,
        &template.require_cell_mark,
        false,
    )?;
    apply_selector_mark_to_cell(
        &mut cell,
        ObjectId::EMPTY,
        None,
        &template.forbid_cell_mark,
        true,
    )?;
    for selector in &template.forbid {
        for object in &selector.alternatives {
            push_unique_object(&mut cell.forbid_objects, *object);
            apply_selector_mark_to_cell(&mut cell, *object, None, &selector.mark, true)?;
        }
    }

    let mut partials = vec![(cell, assignments)];
    for (selector_index, selector) in template.require.iter().enumerate() {
        let mut next = Vec::new();
        for (cell, assignments) in partials {
            if let Some(assigned) = assignments
                .iter()
                .find(|assignment| assignment.token == selector.token)
            {
                match &assigned.value {
                    SelectorAssignmentValue3::Object(object) => {
                        if selector.alternatives.contains(object) {
                            let mut cell = cell;
                            push_unique_object(&mut cell.require_objects, *object);
                            apply_selector_mark_to_cell(
                                &mut cell,
                                *object,
                                None,
                                &selector.mark,
                                false,
                            )?;
                            next.push((cell, assignments));
                        }
                    }
                    SelectorAssignmentValue3::ObjectSet {
                        binding,
                        layer,
                        objects,
                    } => {
                        if objects
                            .iter()
                            .any(|object| selector.alternatives.contains(object))
                        {
                            let mut cell = cell;
                            if !cell
                                .require_object_sets
                                .iter()
                                .any(|existing| existing.binding == *binding)
                            {
                                cell.require_object_sets
                                    .push(puzzle_kernel::ObjectSetMatcher {
                                        binding: *binding,
                                        layer: *layer,
                                        objects: objects.clone(),
                                    });
                            }
                            apply_selector_mark_to_cell(
                                &mut cell,
                                ObjectId::EMPTY,
                                Some(*binding),
                                &selector.mark,
                                false,
                            )?;
                            next.push((cell, assignments));
                        }
                    }
                }
                continue;
            }

            if selector.mark.is_empty()
                && let Some(matcher) = same_layer_object_set_matcher(
                    selector,
                    u16::try_from(assignments.len()).unwrap_or(u16::MAX),
                )
                && match_cell_selector_can_use_object_set(template, selector_index, matcher.layer)
            {
                let mut cell = cell;
                cell.require_object_sets.push(matcher.clone());
                let mut assignments = assignments.clone();
                assignments.push(SelectorAssignment3 {
                    token: selector.token.clone(),
                    value: SelectorAssignmentValue3::ObjectSet {
                        binding: matcher.binding,
                        layer: matcher.layer,
                        objects: matcher.objects,
                    },
                });
                next.push((cell, assignments));
                continue;
            }

            for object in &selector.alternatives {
                let mut cell = cell.clone();
                let mut assignments = assignments.clone();
                push_unique_object(&mut cell.require_objects, *object);
                apply_selector_mark_to_cell(&mut cell, *object, None, &selector.mark, false)?;
                assignments.push(SelectorAssignment3 {
                    token: selector.token.clone(),
                    value: SelectorAssignmentValue3::Object(*object),
                });
                next.push((cell, assignments));
            }
        }
        partials = next;
    }

    Ok(partials
        .into_iter()
        .map(|(cell, assignments)| {
            let mut cells = existing_cells.clone();
            cells.push(cell);
            PatternPartial3 { cells, assignments }
        })
        .collect())
}

fn same_layer_object_set_matcher(
    selector: &ResolvedObjectSelector3,
    binding: u16,
) -> Option<puzzle_kernel::ObjectSetMatcher<ObjectId, LayerId>> {
    if selector.alternatives.len() <= 1 {
        return None;
    }
    Some(puzzle_kernel::ObjectSetMatcher {
        binding,
        layer: selector.runtime_object_set_layer?,
        objects: selector.alternatives.clone(),
    })
}

fn apply_selector_mark_to_cell(
    cell: &mut MatchCell3,
    object: ObjectId,
    binding: Option<u16>,
    marks: &[ResolvedSelectorMark3],
    force_forbid: bool,
) -> Result<(), PatternLoweringError3> {
    for attr in marks {
        let (mark, value, match_value) = (attr.id, attr.value, attr.match_value);
        let negated = force_forbid || attr.negated;
        match (binding, negated) {
            (Some(binding), false) => cell.require_object_set_mark.push(ObjectSetMarkPattern3 {
                binding,
                mark,
                value,
                match_value,
            }),
            (Some(binding), true) => cell.forbid_object_set_mark.push(ObjectSetMarkPattern3 {
                binding,
                mark,
                value,
                match_value,
            }),
            (None, false) => cell.require_mark.push(MarkPattern3 {
                object,
                mark,
                value,
                match_value,
            }),
            (None, true) => cell.forbid_mark.push(MarkPattern3 {
                object,
                mark,
                value,
                match_value,
            }),
        }
    }
    Ok(())
}

fn match_cell_selector_can_use_object_set(
    template: &MatchCellTemplate3<ResolvedObjectSelector3, ResolvedSelectorMark3>,
    selector_index: usize,
    layer: LayerId,
) -> bool {
    !template
        .require
        .iter()
        .enumerate()
        .any(|(other_index, other)| {
            if other_index == selector_index {
                return false;
            }
            other.alternatives.len() > 1
                && same_layer_object_set_matcher(other, 0)
                    .is_none_or(|matcher| matcher.layer == layer)
        })
}

fn lower_write_templates(
    assignments: &[SelectorAssignment3],
    templates: &[WriteOpTemplate3<ResolvedObjectSelector3, ResolvedSelectorMark3>],
) -> Result<Vec<WriteOp3>, RuleLoweringError3> {
    let mut writes = Vec::new();
    for template in templates {
        match template {
            WriteOpTemplate3::Add { offset, object } => match write_object(assignments, object)? {
                WriteObject3::Object(object) => writes.push(WriteOp3::Add {
                    component: 0,
                    offset: offset.clone(),
                    object,
                }),
                WriteObject3::ObjectSet { binding } => writes.push(WriteOp3::AddObjectSet {
                    component: 0,
                    offset: offset.clone(),
                    binding,
                }),
            },
            WriteOpTemplate3::Remove { offset, object } => {
                match write_object(assignments, object)? {
                    WriteObject3::Object(object) => writes.push(WriteOp3::Remove {
                        component: 0,
                        offset: offset.clone(),
                        object,
                    }),
                    WriteObject3::ObjectSet { binding } => writes.push(WriteOp3::RemoveObjectSet {
                        component: 0,
                        offset: offset.clone(),
                        binding,
                    }),
                }
            }
            WriteOpTemplate3::Replace {
                offset,
                remove,
                add,
            } => {
                match (
                    write_object(assignments, remove)?,
                    write_object(assignments, add)?,
                ) {
                    (WriteObject3::Object(remove), WriteObject3::Object(add)) => {
                        writes.push(WriteOp3::Replace {
                            component: 0,
                            offset: offset.clone(),
                            remove,
                            add,
                        });
                    }
                    (WriteObject3::ObjectSet { binding }, WriteObject3::Object(add)) => {
                        writes.push(WriteOp3::RemoveObjectSet {
                            component: 0,
                            offset: offset.clone(),
                            binding,
                        });
                        writes.push(WriteOp3::Add {
                            component: 0,
                            offset: offset.clone(),
                            object: add,
                        });
                    }
                    (WriteObject3::Object(remove), WriteObject3::ObjectSet { binding }) => {
                        writes.push(WriteOp3::Remove {
                            component: 0,
                            offset: offset.clone(),
                            object: remove,
                        });
                        writes.push(WriteOp3::AddObjectSet {
                            component: 0,
                            offset: offset.clone(),
                            binding,
                        });
                    }
                    (
                        WriteObject3::ObjectSet {
                            binding: remove_binding,
                        },
                        WriteObject3::ObjectSet {
                            binding: add_binding,
                        },
                    ) => {
                        writes.push(WriteOp3::RemoveObjectSet {
                            component: 0,
                            offset: offset.clone(),
                            binding: remove_binding,
                        });
                        writes.push(WriteOp3::AddObjectSet {
                            component: 0,
                            offset: offset.clone(),
                            binding: add_binding,
                        });
                    }
                }
            }
            WriteOpTemplate3::Move {
                from_offset,
                to_offset,
                object,
            } => match write_object(assignments, object)? {
                WriteObject3::Object(object) => writes.push(WriteOp3::Move {
                    component: 0,
                    from_offset: from_offset.clone(),
                    to_offset: to_offset.clone(),
                    object,
                }),
                WriteObject3::ObjectSet { binding } => writes.push(WriteOp3::MoveObjectSet {
                    component: 0,
                    from_offset: from_offset.clone(),
                    to_offset: to_offset.clone(),
                    binding,
                }),
            },
            WriteOpTemplate3::SetMark {
                offset,
                object,
                mark,
            } => match write_object(assignments, object)? {
                WriteObject3::Object(object) => {
                    let (mark, value) = (mark.id, mark.value);
                    writes.push(WriteOp3::SetMark {
                        component: 0,
                        offset: offset.clone(),
                        object,
                        mark,
                        value,
                    });
                }
                WriteObject3::ObjectSet { binding } => {
                    let (mark, value) = (mark.id, mark.value);
                    writes.push(WriteOp3::SetObjectSetMark {
                        component: 0,
                        offset: offset.clone(),
                        binding,
                        mark,
                        value,
                    });
                }
            },
            WriteOpTemplate3::RemoveMark {
                offset,
                object,
                mark,
            } => match write_object(assignments, object)? {
                WriteObject3::Object(object) => {
                    let (mark, value, match_value) = (mark.id, mark.value, mark.match_value);
                    writes.push(WriteOp3::RemoveMark {
                        component: 0,
                        offset: offset.clone(),
                        object,
                        mark,
                        value,
                        match_value,
                    });
                }
                WriteObject3::ObjectSet { binding } => {
                    let (mark, value, match_value) = (mark.id, mark.value, mark.match_value);
                    writes.push(WriteOp3::RemoveObjectSetMark {
                        component: 0,
                        offset: offset.clone(),
                        binding,
                        mark,
                        value,
                        match_value,
                    });
                }
            },
        }
    }
    Ok(writes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WriteObject3 {
    Object(ObjectId),
    ObjectSet { binding: u16 },
}

fn write_object(
    assignments: &[SelectorAssignment3],
    selector: &ResolvedObjectSelector3,
) -> Result<WriteObject3, RuleLoweringError3> {
    let token = selector.token.clone();
    if let Some(value) = assignments
        .iter()
        .find(|assignment| assignment.token == token)
        .map(|assignment| &assignment.value)
    {
        return Ok(match value {
            SelectorAssignmentValue3::Object(object) => WriteObject3::Object(*object),
            SelectorAssignmentValue3::ObjectSet { binding, .. } => {
                WriteObject3::ObjectSet { binding: *binding }
            }
        });
    }

    if selector.occurrence_labeled {
        return Err(RuleLoweringError3::UnboundSelectorOccurrenceLabel { token });
    }

    if selector.alternatives.len() == 1 {
        return Ok(WriteObject3::Object(selector.alternatives[0]));
    }

    Err(RuleLoweringError3::AmbiguousWriteSelector {
        token,
        alternatives: selector.alternatives.clone(),
    })
}

fn resolve_directional_selector_mark(mark: &SelectorMark3, direction: Direction3) -> SelectorMark3 {
    let Some(value) = mark.value.as_deref() else {
        return mark.clone();
    };
    let value = match value {
        ">" => direction.name,
        "<" => direction.opposite().name,
        other => puzzle_authoring::canonical_3d_movement_direction_name(other),
    };
    SelectorMark3 {
        name: mark.name.clone(),
        value: Some(value.to_string()),
        negated: mark.negated,
    }
}

fn resolve_directional_object_selector3(
    selector: &ObjectSelector3,
    direction: Direction3,
) -> ObjectSelector3 {
    match selector {
        ObjectSelector3::WithMark { selector, mark } => ObjectSelector3::with_mark(
            resolve_directional_object_selector3(selector, direction),
            mark.iter()
                .map(|mark| resolve_directional_selector_mark(mark, direction))
                .collect(),
        ),
        ObjectSelector3::Labeled { token, selector } => {
            let selector = resolve_directional_object_selector3(selector, direction);
            let label = token.rsplit_once('#').map_or("", |(_, label)| label);
            ObjectSelector3::labeled(format!("{}#{label}", selector.token()), selector)
        }
        ObjectSelector3::Variant { family, tags } => ObjectSelector3::variant(
            family,
            tags.iter()
                .map(|tag| match tag {
                    SelectorTag3::Value(value) if value == ">" => {
                        SelectorTag3::value(direction.name)
                    }
                    SelectorTag3::Value(value) if value == "<" => {
                        SelectorTag3::value(direction.opposite().name)
                    }
                    _ => tag.clone(),
                })
                .collect(),
        ),
        _ => selector.clone(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternLoweringError3 {
    DuplicateSelectorOccurrenceLabel { token: String },
    InvalidMark { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleLoweringError3 {
    Pattern(PatternLoweringError3),
    UnboundSelectorOccurrenceLabel {
        token: String,
    },
    AmbiguousWriteSelector {
        token: String,
        alternatives: Vec<ObjectId>,
    },
    InvalidMark {
        name: String,
    },
}

impl From<PatternLoweringError3> for RuleLoweringError3 {
    fn from(value: PatternLoweringError3) -> Self {
        Self::Pattern(value)
    }
}

fn push_unique_object(objects: &mut Vec<ObjectId>, object: ObjectId) {
    if !objects.contains(&object) {
        objects.push(object);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(object: ObjectId) -> ResolvedObjectSelector3 {
        ResolvedObjectSelector3 {
            token: format!("object_{}", object.0),
            alternatives: vec![object],
            mark: Vec::new(),
            occurrence_labeled: false,
            runtime_object_set_layer: None,
        }
    }

    fn resolved_mark(id: u16, value: Option<i64>) -> ResolvedSelectorMark3 {
        ResolvedSelectorMark3 {
            id: MarkId3(id),
            value,
            match_value: puzzle_kernel::MarkValueMatch::Exact,
            negated: false,
        }
    }

    #[test]
    fn line_projection_preserves_variable_gap_offsets() {
        let mut pattern = LinePatternTemplate3::new(vec![LineMatchCellTemplate3 {
            step: LineOffsetTemplate3 {
                base: 1,
                gap_terms: vec![0],
            },
            require_null: false,
            require: Vec::new(),
            forbid: Vec::new(),
            require_cell_mark: Vec::new(),
            forbid_cell_mark: Vec::new(),
        }]);
        pattern.gap_count = 1;
        let projected = project_line_rule_template(&LineRuleTemplate3::once(
            LineOrientation3::Direction(Direction3::RIGHT),
            pattern,
            Vec::new(),
        ));

        assert_eq!(projected[0].pattern.gap_count, 1);
        assert_eq!(
            projected[0].pattern.cells[0].offset,
            Offset3::Variable {
                base_dx: 1,
                base_dy: 0,
                base_dz: 0,
                gap_terms: vec![GapTerm3 {
                    gap_index: 0,
                    dx: 1,
                    dy: 0,
                    dz: 0,
                }],
            }
        );
    }

    #[test]
    fn resolved_named_and_cell_marks_materialize_into_shared_rule_contract() {
        let player = ObjectId(1);
        let mut selector = resolved(player);
        selector.mark.push(resolved_mark(7, Some(3)));
        let mut cell = MatchCellTemplate3::new(Delta3::ZERO).require(selector.clone());
        cell.require_cell_mark.push(resolved_mark(8, Some(5)));
        let rule = RuleTemplate3::once(
            PatternTemplate3::new(vec![cell]),
            vec![WriteOpTemplate3::SetMark {
                offset: Delta3::ZERO.into(),
                object: selector,
                mark: resolved_mark(9, Some(11)),
            }],
        );

        let lowered = lower_rule_template(&rule).unwrap();
        let cell = lowered[0].pattern.cells()[0];
        assert!(cell.require_mark.iter().any(|mark| {
            mark.object == player && mark.mark == MarkId3(7) && mark.value == Some(3)
        }));
        assert!(cell.require_mark.iter().any(|mark| {
            mark.object == ObjectId::EMPTY && mark.mark == MarkId3(8) && mark.value == Some(5)
        }));
        assert!(matches!(
            lowered[0].writes.as_slice(),
            [WriteOp3::SetMark {
                object,
                mark: MarkId3(9),
                value: Some(11),
                ..
            }] if *object == player
        ));
    }

    #[test]
    fn line_projection_expands_orientation_and_relative_selector_tags() {
        let rule = LineRuleTemplate3::once(
            LineOrientation3::DirectionSet(DirectionSet3::Horizontal),
            LinePatternTemplate3::new(vec![LineMatchCellTemplate3::new(1).require(
                ObjectSelector3::variant("Marker", vec![SelectorTag3::value(">")]),
            )]),
            Vec::new(),
        );

        let projected = project_line_rule_template(&rule);

        assert_eq!(projected.len(), 4);
        for (rule, direction) in projected.iter().zip(DirectionSet3::Horizontal.directions()) {
            assert_eq!(rule.pattern.cells[0].offset, direction.offset.into());
            assert_eq!(
                rule.pattern.cells[0].require[0].token(),
                format!("Marker:{}", direction.name)
            );
        }
    }

    #[test]
    fn dense_projection_transforms_pattern_and_write_offsets_through_frame() {
        let rule = DenseRuleTemplate3::once(
            FrameOrientation3::Frame(Frame3::DEFAULT),
            DensePattern3::new(vec![DenseSlice3::new(vec![DenseRow3::new(vec![
                DenseCell3::empty(),
                DenseCell3::require(ObjectSelector3::object("Player")),
            ])])]),
            vec![LocalWriteOpTemplate3::Move {
                from_offset: Delta3::ZERO,
                to_offset: Delta3::new(1, 0, 0),
                object: ObjectSelector3::object("Player"),
            }],
        );

        let projected = project_dense_rule_template(&rule);

        assert_eq!(projected.len(), 1);
        assert_eq!(
            projected[0].pattern.cells[0].offset,
            Frame3::DEFAULT.primary.offset.into()
        );
        assert!(matches!(
            projected[0].writes.as_slice(),
            [WriteOpTemplate3::Move {
                from_offset,
                to_offset,
                ..
            }] if *from_offset == Delta3::ZERO.into()
                && *to_offset == Frame3::DEFAULT.primary.offset.into()
        ));
    }

    #[test]
    fn resolved_rule_lowering_requires_no_selector_catalog() {
        let player = ObjectId(1);
        let rule = RuleTemplate3::once(
            PatternTemplate3::new(vec![
                MatchCellTemplate3::new(Delta3::ZERO).require(resolved(player)),
            ]),
            vec![WriteOpTemplate3::Move {
                from_offset: Delta3::ZERO.into(),
                to_offset: Direction3::RIGHT.offset.into(),
                object: resolved(player),
            }],
        );

        let lowered = lower_rule_template(&rule).unwrap();

        assert_eq!(lowered.len(), 1);
        assert_eq!(lowered[0].pattern.cells()[0].require_objects, vec![player]);
        assert_eq!(
            lowered[0].writes,
            vec![WriteOp3::Move {
                component: 0,
                from_offset: Delta3::ZERO.into(),
                to_offset: Direction3::RIGHT.offset.into(),
                object: player,
            }]
        );
    }
}
