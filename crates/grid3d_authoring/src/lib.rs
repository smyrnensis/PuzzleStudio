use puzzle_grid3d::{
    Direction3, DirectionSet3, Frame3, FrameSet3, Guard3, InputId, LayerId, MarkId3, MarkPattern3,
    MatchCell3, ObjectId, ObjectSetMarkPattern3, Offset3, Pattern3, Rule3, RuleApplication3,
    RuleId3, WriteOp3,
};

pub type SelectorCatalog3 =
    puzzle_authoring::SelectorCatalog<ObjectId, LayerId, VariantAxis3, SelectorMark3>;
pub type SelectorMark3 = puzzle_authoring::SelectorMark;
pub type ConcreteObject3 = puzzle_authoring::ConcreteObject<ObjectId>;
pub type SelectorGroup3 = puzzle_authoring::SelectorGroup<ObjectSelector3>;
pub type ObjectFamily3 = puzzle_authoring::ObjectFamily<ObjectId, VariantAxis3>;
pub type ObjectVariant3 = puzzle_authoring::ObjectVariant<ObjectId>;
pub type ObjectSelector3 = puzzle_authoring::ObjectSelector<SelectorMark3>;
pub type SelectorTag3 = puzzle_authoring::SelectorTag;
pub type SelectorCatalogError3 = puzzle_authoring::SelectorCatalogError;
pub type SelectorError3 = puzzle_authoring::SelectorError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatternTemplate3 {
    pub cells: Vec<MatchCellTemplate3>,
}

impl PatternTemplate3 {
    pub fn new(cells: Vec<MatchCellTemplate3>) -> Self {
        Self { cells }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchCellTemplate3 {
    pub offset: Offset3,
    pub require: Vec<ObjectSelector3>,
    pub forbid: Vec<ObjectSelector3>,
}

impl MatchCellTemplate3 {
    pub fn new(offset: Offset3) -> Self {
        Self {
            offset,
            require: Vec::new(),
            forbid: Vec::new(),
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
    pub require: Vec<ObjectSelector3>,
    pub forbid: Vec<ObjectSelector3>,
}

impl DenseCell3 {
    pub fn empty() -> Self {
        Self {
            require: Vec::new(),
            forbid: Vec::new(),
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
        self.require.is_empty() && self.forbid.is_empty()
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
                let local = Offset3::new(column as i16, row as i16, depth as i16);
                cells.push(MatchCellTemplate3 {
                    offset: frame.to_world_offset(local),
                    require: dense_cell.require.clone(),
                    forbid: dense_cell.forbid.clone(),
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

pub fn lower_dense_pattern_to_patterns(
    catalog: &SelectorCatalog3,
    frame: Frame3,
    dense: &DensePattern3,
) -> Result<Vec<Pattern3>, PatternLoweringError3> {
    lower_pattern_template(catalog, &lower_dense_pattern(frame, dense))
}

pub fn lower_dense_pattern_set_to_patterns(
    catalog: &SelectorCatalog3,
    frames: FrameSet3,
    dense: &DensePattern3,
) -> Result<Vec<Pattern3>, PatternLoweringError3> {
    let mut patterns = Vec::new();
    for template in lower_dense_pattern_set(frames, dense) {
        patterns.extend(lower_pattern_template(catalog, &template)?);
    }
    Ok(patterns)
}

pub fn lower_pattern_template(
    catalog: &SelectorCatalog3,
    template: &PatternTemplate3,
) -> Result<Vec<Pattern3>, PatternLoweringError3> {
    Ok(lower_pattern_template_with_assignments(catalog, template)?
        .into_iter()
        .map(|partial| Pattern3::new(partial.cells))
        .collect())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleTemplate3 {
    pub id: RuleId3,
    pub guards: Vec<Guard3>,
    pub application: RuleApplication3,
    pub pattern: PatternTemplate3,
    pub writes: Vec<WriteOpTemplate3>,
}

impl RuleTemplate3 {
    pub fn once(pattern: PatternTemplate3, writes: Vec<WriteOpTemplate3>) -> Self {
        Self {
            id: RuleId3(0),
            guards: Vec::new(),
            application: RuleApplication3::Once,
            pattern,
            writes,
        }
    }

    pub fn once_all(pattern: PatternTemplate3, writes: Vec<WriteOpTemplate3>) -> Self {
        Self {
            id: RuleId3(0),
            guards: Vec::new(),
            application: RuleApplication3::OnceAll,
            pattern,
            writes,
        }
    }

    pub fn once_per_level(pattern: PatternTemplate3, writes: Vec<WriteOpTemplate3>) -> Self {
        Self {
            id: RuleId3(0),
            guards: Vec::new(),
            application: RuleApplication3::OncePerLevel,
            pattern,
            writes,
        }
    }

    pub fn repeated(pattern: PatternTemplate3, writes: Vec<WriteOpTemplate3>) -> Self {
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
pub enum WriteOpTemplate3 {
    Add {
        offset: Offset3,
        object: ObjectSelector3,
    },
    Remove {
        offset: Offset3,
        object: ObjectSelector3,
    },
    Replace {
        offset: Offset3,
        remove: ObjectSelector3,
        add: ObjectSelector3,
    },
    Move {
        from_offset: Offset3,
        to_offset: Offset3,
        object: ObjectSelector3,
    },
    SetMark {
        offset: Offset3,
        object: ObjectSelector3,
        mark: SelectorMark3,
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
}

impl LinePatternTemplate3 {
    pub fn new(cells: Vec<LineMatchCellTemplate3>) -> Self {
        Self { cells }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineMatchCellTemplate3 {
    pub step: i16,
    pub require: Vec<ObjectSelector3>,
    pub forbid: Vec<ObjectSelector3>,
}

impl LineMatchCellTemplate3 {
    pub fn new(step: i16) -> Self {
        Self {
            step,
            require: Vec::new(),
            forbid: Vec::new(),
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
pub enum LineWriteOpTemplate3 {
    Add {
        step: i16,
        object: ObjectSelector3,
    },
    Remove {
        step: i16,
        object: ObjectSelector3,
    },
    Replace {
        step: i16,
        remove: ObjectSelector3,
        add: ObjectSelector3,
    },
    Move {
        from_step: i16,
        to_step: i16,
        object: ObjectSelector3,
    },
    SetMark {
        step: i16,
        object: ObjectSelector3,
        mark: SelectorMark3,
    },
}

pub fn lower_line_rule_template(
    catalog: &SelectorCatalog3,
    template: &LineRuleTemplate3,
) -> Result<Vec<Rule3>, RuleLoweringError3> {
    let mut rules = Vec::new();
    for direction in template.orientation.directions() {
        let pattern = PatternTemplate3::new(
            template
                .pattern
                .cells
                .iter()
                .map(|cell| MatchCellTemplate3 {
                    offset: direction.offset.scale(cell.step),
                    require: cell
                        .require
                        .iter()
                        .map(|selector| {
                            resolve_directional_object_selector3_mark(selector, direction)
                        })
                        .collect(),
                    forbid: cell
                        .forbid
                        .iter()
                        .map(|selector| {
                            resolve_directional_object_selector3_mark(selector, direction)
                        })
                        .collect(),
                })
                .collect(),
        );
        let writes = template
            .writes
            .iter()
            .map(|write| line_write_to_world(direction, write))
            .collect();
        rules.extend(lower_rule_template(
            catalog,
            &RuleTemplate3 {
                id: template.id,
                guards: template.guards.clone(),
                application: template.application,
                pattern,
                writes,
            },
        )?);
    }
    Ok(rules)
}

fn line_write_to_world(direction: Direction3, write: &LineWriteOpTemplate3) -> WriteOpTemplate3 {
    match write {
        LineWriteOpTemplate3::Add { step, object } => WriteOpTemplate3::Add {
            offset: direction.offset.scale(*step),
            object: object.clone(),
        },
        LineWriteOpTemplate3::Remove { step, object } => WriteOpTemplate3::Remove {
            offset: direction.offset.scale(*step),
            object: object.clone(),
        },
        LineWriteOpTemplate3::Replace { step, remove, add } => WriteOpTemplate3::Replace {
            offset: direction.offset.scale(*step),
            remove: remove.clone(),
            add: add.clone(),
        },
        LineWriteOpTemplate3::Move {
            from_step,
            to_step,
            object,
        } => WriteOpTemplate3::Move {
            from_offset: direction.offset.scale(*from_step),
            to_offset: direction.offset.scale(*to_step),
            object: object.clone(),
        },
        LineWriteOpTemplate3::SetMark { step, object, mark } => WriteOpTemplate3::SetMark {
            offset: direction.offset.scale(*step),
            object: object.clone(),
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
        offset: Offset3,
        object: ObjectSelector3,
    },
    Remove {
        offset: Offset3,
        object: ObjectSelector3,
    },
    Replace {
        offset: Offset3,
        remove: ObjectSelector3,
        add: ObjectSelector3,
    },
    Move {
        from_offset: Offset3,
        to_offset: Offset3,
        object: ObjectSelector3,
    },
}

pub fn lower_dense_rule_template(
    catalog: &SelectorCatalog3,
    template: &DenseRuleTemplate3,
) -> Result<Vec<Rule3>, RuleLoweringError3> {
    let mut rules = Vec::new();
    for frame in template.orientation.frames() {
        let pattern = lower_dense_pattern(frame, &template.pattern);
        let writes = template
            .writes
            .iter()
            .map(|write| local_write_to_world(frame, write))
            .collect();
        rules.extend(lower_rule_template(
            catalog,
            &RuleTemplate3 {
                id: template.id,
                guards: template.guards.clone(),
                application: template.application,
                pattern,
                writes,
            },
        )?);
    }
    Ok(rules)
}

fn local_write_to_world(frame: Frame3, write: &LocalWriteOpTemplate3) -> WriteOpTemplate3 {
    match write {
        LocalWriteOpTemplate3::Add { offset, object } => WriteOpTemplate3::Add {
            offset: frame.to_world_offset(*offset),
            object: object.clone(),
        },
        LocalWriteOpTemplate3::Remove { offset, object } => WriteOpTemplate3::Remove {
            offset: frame.to_world_offset(*offset),
            object: object.clone(),
        },
        LocalWriteOpTemplate3::Replace {
            offset,
            remove,
            add,
        } => WriteOpTemplate3::Replace {
            offset: frame.to_world_offset(*offset),
            remove: remove.clone(),
            add: add.clone(),
        },
        LocalWriteOpTemplate3::Move {
            from_offset,
            to_offset,
            object,
        } => WriteOpTemplate3::Move {
            from_offset: frame.to_world_offset(*from_offset),
            to_offset: frame.to_world_offset(*to_offset),
            object: object.clone(),
        },
    }
}

pub fn lower_rule_template(
    catalog: &SelectorCatalog3,
    template: &RuleTemplate3,
) -> Result<Vec<Rule3>, RuleLoweringError3> {
    let pattern_partials = lower_pattern_template_with_assignments(catalog, &template.pattern)?;
    pattern_partials
        .into_iter()
        .map(|partial| {
            let writes = lower_write_templates(catalog, &partial.assignments, &template.writes)?;
            Ok(Rule3 {
                id: template.id,
                guards: template.guards.clone(),
                application: template.application,
                pattern: Pattern3::new(partial.cells),
                writes,
                effects: Vec::new(),
            })
        })
        .collect()
}

fn lower_pattern_template_with_assignments(
    catalog: &SelectorCatalog3,
    template: &PatternTemplate3,
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
            next_partials.extend(lower_match_cell_template(catalog, cell, partial)?);
        }
        partials = next_partials;
    }

    if has_labeled_selectors {
        partials.reverse();
    }
    Ok(partials)
}

fn pattern_template_has_labeled_selectors(template: &PatternTemplate3) -> bool {
    template.cells.iter().any(|cell| {
        cell.require
            .iter()
            .any(ObjectSelector3::has_occurrence_label)
    })
}

fn reject_duplicate_labeled_selectors(
    template: &PatternTemplate3,
) -> Result<(), PatternLoweringError3> {
    let mut seen = Vec::<String>::new();
    for cell in &template.cells {
        for selector in &cell.require {
            if !selector.has_occurrence_label() {
                continue;
            }
            let token = selector.token();
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
    catalog: &SelectorCatalog3,
    template: &MatchCellTemplate3,
    partial: PatternPartial3,
) -> Result<Vec<PatternPartial3>, PatternLoweringError3> {
    let PatternPartial3 {
        cells: existing_cells,
        assignments,
    } = partial;
    let mut cell = MatchCell3::new(template.offset);
    for selector in &template.forbid {
        let resolved = catalog.resolve(selector)?;
        for object in resolved.alternatives {
            push_unique_object(&mut cell.forbid_objects, object);
            apply_selector_mark_to_cell(&mut cell, object, None, &resolved.mark, true)?;
        }
    }

    let mut partials = vec![(cell, assignments)];
    for (selector_index, selector) in template.require.iter().enumerate() {
        let resolved = catalog.resolve(selector)?;
        let mut next = Vec::new();
        for (cell, assignments) in partials {
            if let Some(assigned) = assignments
                .iter()
                .find(|assignment| assignment.token == resolved.token)
            {
                match &assigned.value {
                    SelectorAssignmentValue3::Object(object) => {
                        if resolved.alternatives.contains(object) {
                            let mut cell = cell;
                            push_unique_object(&mut cell.require_objects, *object);
                            apply_selector_mark_to_cell(
                                &mut cell,
                                *object,
                                None,
                                &resolved.mark,
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
                            .any(|object| resolved.alternatives.contains(object))
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
                                &resolved.mark,
                                false,
                            )?;
                            next.push((cell, assignments));
                        }
                    }
                }
                continue;
            }

            if resolved.mark.is_empty()
                && selector.can_use_runtime_object_set()
                && let Some(matcher) = same_layer_object_set_matcher(
                    catalog,
                    u16::try_from(assignments.len()).unwrap_or(u16::MAX),
                    &resolved.alternatives,
                )
                && match_cell_selector_can_use_object_set(
                    catalog,
                    template,
                    selector_index,
                    matcher.layer,
                )
            {
                let mut cell = cell;
                cell.require_object_sets.push(matcher.clone());
                let mut assignments = assignments.clone();
                assignments.push(SelectorAssignment3 {
                    token: resolved.token.clone(),
                    value: SelectorAssignmentValue3::ObjectSet {
                        binding: matcher.binding,
                        layer: matcher.layer,
                        objects: matcher.objects,
                    },
                });
                next.push((cell, assignments));
                continue;
            }

            for object in &resolved.alternatives {
                let mut cell = cell.clone();
                let mut assignments = assignments.clone();
                push_unique_object(&mut cell.require_objects, *object);
                apply_selector_mark_to_cell(&mut cell, *object, None, &resolved.mark, false)?;
                assignments.push(SelectorAssignment3 {
                    token: resolved.token.clone(),
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
    catalog: &SelectorCatalog3,
    binding: u16,
    alternatives: &[ObjectId],
) -> Option<puzzle_kernel::ObjectSetMatcher<ObjectId, LayerId>> {
    if alternatives.len() <= 1 {
        return None;
    }
    puzzle_kernel::object_set_matcher_for_same_layer(binding, alternatives, |object| {
        catalog.object_layer(object)
    })
}

fn apply_selector_mark_to_cell(
    cell: &mut MatchCell3,
    object: ObjectId,
    binding: Option<u16>,
    marks: &[SelectorMark3],
    force_forbid: bool,
) -> Result<(), PatternLoweringError3> {
    for attr in marks {
        let (mark, value, match_value) = lower_selector_mark_for_pattern(attr)?;
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
    catalog: &SelectorCatalog3,
    template: &MatchCellTemplate3,
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
            let Ok(resolved) = catalog.resolve(other) else {
                return true;
            };
            resolved.alternatives.len() > 1
                && same_layer_object_set_matcher(catalog, 0, &resolved.alternatives)
                    .is_none_or(|matcher| matcher.layer == layer)
        })
}

fn lower_write_templates(
    catalog: &SelectorCatalog3,
    assignments: &[SelectorAssignment3],
    templates: &[WriteOpTemplate3],
) -> Result<Vec<WriteOp3>, RuleLoweringError3> {
    let mut writes = Vec::new();
    for template in templates {
        match template {
            WriteOpTemplate3::Add { offset, object } => {
                match write_object(catalog, assignments, object)? {
                    WriteObject3::Object(object) => writes.push(WriteOp3::Add {
                        component: 0,
                        offset: *offset,
                        object,
                    }),
                    WriteObject3::ObjectSet { binding } => writes.push(WriteOp3::AddObjectSet {
                        component: 0,
                        offset: *offset,
                        binding,
                    }),
                }
            }
            WriteOpTemplate3::Remove { offset, object } => {
                match write_object(catalog, assignments, object)? {
                    WriteObject3::Object(object) => writes.push(WriteOp3::Remove {
                        component: 0,
                        offset: *offset,
                        object,
                    }),
                    WriteObject3::ObjectSet { binding } => writes.push(WriteOp3::RemoveObjectSet {
                        component: 0,
                        offset: *offset,
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
                    write_object(catalog, assignments, remove)?,
                    write_object(catalog, assignments, add)?,
                ) {
                    (WriteObject3::Object(remove), WriteObject3::Object(add)) => {
                        writes.push(WriteOp3::Replace {
                            component: 0,
                            offset: *offset,
                            remove,
                            add,
                        });
                    }
                    (WriteObject3::ObjectSet { binding }, WriteObject3::Object(add)) => {
                        writes.push(WriteOp3::RemoveObjectSet {
                            component: 0,
                            offset: *offset,
                            binding,
                        });
                        writes.push(WriteOp3::Add {
                            component: 0,
                            offset: *offset,
                            object: add,
                        });
                    }
                    (WriteObject3::Object(remove), WriteObject3::ObjectSet { binding }) => {
                        writes.push(WriteOp3::Remove {
                            component: 0,
                            offset: *offset,
                            object: remove,
                        });
                        writes.push(WriteOp3::AddObjectSet {
                            component: 0,
                            offset: *offset,
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
                            offset: *offset,
                            binding: remove_binding,
                        });
                        writes.push(WriteOp3::AddObjectSet {
                            component: 0,
                            offset: *offset,
                            binding: add_binding,
                        });
                    }
                }
            }
            WriteOpTemplate3::Move {
                from_offset,
                to_offset,
                object,
            } => match write_object(catalog, assignments, object)? {
                WriteObject3::Object(object) => writes.push(WriteOp3::Move {
                    component: 0,
                    from_offset: *from_offset,
                    to_offset: *to_offset,
                    object,
                }),
                WriteObject3::ObjectSet { binding } => writes.push(WriteOp3::MoveObjectSet {
                    component: 0,
                    from_offset: *from_offset,
                    to_offset: *to_offset,
                    binding,
                }),
            },
            WriteOpTemplate3::SetMark {
                offset,
                object,
                mark,
            } => match write_object(catalog, assignments, object)? {
                WriteObject3::Object(object) => {
                    let (mark, value, _) = lower_selector_mark(mark)?;
                    writes.push(WriteOp3::SetMark {
                        component: 0,
                        offset: *offset,
                        object,
                        mark,
                        value,
                    });
                }
                WriteObject3::ObjectSet { binding } => {
                    let (mark, value, _) = lower_selector_mark(mark)?;
                    writes.push(WriteOp3::SetObjectSetMark {
                        component: 0,
                        offset: *offset,
                        binding,
                        mark,
                        value,
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
    catalog: &SelectorCatalog3,
    assignments: &[SelectorAssignment3],
    selector: &ObjectSelector3,
) -> Result<WriteObject3, RuleLoweringError3> {
    let token = selector.token();
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

    if selector.has_occurrence_label() {
        return Err(RuleLoweringError3::UnboundSelectorOccurrenceLabel { token });
    }

    let resolved = catalog.resolve(selector)?;
    if resolved.alternatives.len() == 1 {
        return Ok(WriteObject3::Object(resolved.alternatives[0]));
    }

    Err(RuleLoweringError3::AmbiguousWriteSelector {
        token: resolved.token,
        alternatives: resolved.alternatives,
    })
}

const ANONYMOUS_MOVEMENT_MARK3: MarkId3 = MarkId3(puzzle_authoring::ANONYMOUS_MOVEMENT_MARK_INDEX);

fn lower_selector_mark(
    mark: &SelectorMark3,
) -> Result<(MarkId3, Option<i64>, puzzle_kernel::MarkValueMatch), RuleLoweringError3> {
    lower_selector_mark_parts(mark).map_err(|name| RuleLoweringError3::InvalidMark { name })
}

fn lower_selector_mark_for_pattern(
    mark: &SelectorMark3,
) -> Result<(MarkId3, Option<i64>, puzzle_kernel::MarkValueMatch), PatternLoweringError3> {
    lower_selector_mark_parts(mark).map_err(|name| PatternLoweringError3::InvalidMark { name })
}

fn lower_selector_mark_parts(
    mark: &SelectorMark3,
) -> Result<(MarkId3, Option<i64>, puzzle_kernel::MarkValueMatch), String> {
    if mark.name.is_empty()
        && let Some(value) = mark.value.as_deref()
    {
        if value == "directions" {
            return Ok((
                ANONYMOUS_MOVEMENT_MARK3,
                None,
                puzzle_kernel::MarkValueMatch::Any,
            ));
        }
        let value =
            puzzle_authoring::movement_mark_index_3d(value).ok_or_else(|| value.to_string())?;
        return Ok((
            ANONYMOUS_MOVEMENT_MARK3,
            Some(i64::from(value)),
            puzzle_kernel::MarkValueMatch::Exact,
        ));
    }
    Err(mark.name.clone())
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

fn resolve_directional_object_selector3_mark(
    selector: &ObjectSelector3,
    direction: Direction3,
) -> ObjectSelector3 {
    match selector {
        ObjectSelector3::WithMark { selector, mark } => ObjectSelector3::with_mark(
            resolve_directional_object_selector3_mark(selector, direction),
            mark.iter()
                .map(|mark| resolve_directional_selector_mark(mark, direction))
                .collect(),
        ),
        ObjectSelector3::Labeled { token, selector } => ObjectSelector3::Labeled {
            token: token.clone(),
            selector: Box::new(resolve_directional_object_selector3_mark(
                selector, direction,
            )),
        },
        _ => selector.clone(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantAxis3 {
    pub name: String,
    pub values: VariantValueSet3,
}

impl VariantAxis3 {
    pub fn named(name: impl Into<String>, values: Vec<impl Into<String>>) -> Self {
        Self {
            name: name.into(),
            values: VariantValueSet3::Named(values.into_iter().map(Into::into).collect()),
        }
    }

    pub fn directions(name: impl Into<String>, set: DirectionSet3) -> Self {
        Self {
            name: name.into(),
            values: VariantValueSet3::Directions(set),
        }
    }
}

impl puzzle_authoring::VariantAxisSpec for VariantAxis3 {
    fn name(&self) -> &str {
        &self.name
    }

    fn allowed_values(&self, tag: &str) -> Option<Vec<String>> {
        match &self.values {
            VariantValueSet3::Named(values) => values
                .iter()
                .any(|value| value == tag)
                .then(|| vec![tag.to_string()]),
            VariantValueSet3::Directions(set) => {
                let requested = requested_direction_values(tag)?;
                let axis_values = direction_values(*set);
                let values = requested
                    .into_iter()
                    .filter(|value| axis_values.contains(value))
                    .collect::<Vec<_>>();
                (!values.is_empty()).then_some(values)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VariantValueSet3 {
    Named(Vec<String>),
    Directions(DirectionSet3),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternLoweringError3 {
    Selector(SelectorError3),
    DuplicateSelectorOccurrenceLabel { token: String },
    InvalidMark { name: String },
}

impl From<SelectorError3> for PatternLoweringError3 {
    fn from(value: SelectorError3) -> Self {
        Self::Selector(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleLoweringError3 {
    Pattern(PatternLoweringError3),
    Selector(SelectorError3),
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

impl From<SelectorError3> for RuleLoweringError3 {
    fn from(value: SelectorError3) -> Self {
        Self::Selector(value)
    }
}

fn push_unique_object(objects: &mut Vec<ObjectId>, object: ObjectId) {
    if !objects.contains(&object) {
        objects.push(object);
    }
}

fn requested_direction_values(tag: &str) -> Option<Vec<String>> {
    if let Some(direction) = Direction3::by_name(tag) {
        return Some(vec![direction.name.to_string()]);
    }
    match tag {
        "directions" => Some(direction_values(DirectionSet3::Directions)),
        "horizontal" => Some(direction_values(DirectionSet3::Horizontal)),
        "vertical" => Some(direction_values(DirectionSet3::Vertical)),
        _ => None,
    }
}

fn direction_values(set: DirectionSet3) -> Vec<String> {
    set.directions()
        .into_iter()
        .map(|direction| direction.name.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAYER: ObjectId = ObjectId(1);
    const BOX: ObjectId = ObjectId(2);
    const WALL: ObjectId = ObjectId(3);
    const MARKER_LEFT: ObjectId = ObjectId(10);
    const MARKER_RIGHT: ObjectId = ObjectId(11);
    const MARKER_FORWARD: ObjectId = ObjectId(12);
    const MARKER_BACKWARD: ObjectId = ObjectId(13);
    const MARKER_UP: ObjectId = ObjectId(14);
    const MARKER_DOWN: ObjectId = ObjectId(15);
    const INPUT_RIGHT: InputId = InputId(1);

    fn selector_catalog() -> SelectorCatalog3 {
        SelectorCatalog3::new(
            vec![
                ConcreteObject3::new(PLAYER, "Player"),
                ConcreteObject3::new(BOX, "Box"),
                ConcreteObject3::new(WALL, "Wall"),
            ],
            vec![ObjectFamily3::new(
                "Marker",
                vec![VariantAxis3::directions(
                    "direction",
                    DirectionSet3::Directions,
                )],
                vec![
                    ObjectVariant3::new(MARKER_LEFT, vec!["left"]),
                    ObjectVariant3::new(MARKER_RIGHT, vec!["right"]),
                    ObjectVariant3::new(MARKER_FORWARD, vec!["front"]),
                    ObjectVariant3::new(MARKER_BACKWARD, vec!["back"]),
                    ObjectVariant3::new(MARKER_UP, vec!["up"]),
                    ObjectVariant3::new(MARKER_DOWN, vec!["down"]),
                ],
            )],
            vec![SelectorGroup3::new(
                "solid",
                vec![
                    ObjectSelector3::object("Player"),
                    ObjectSelector3::object("Box"),
                    ObjectSelector3::object("Wall"),
                    ObjectSelector3::object("Box"),
                ],
            )],
        )
    }

    #[test]
    fn group_selector_can_expand_nested_selectors() {
        let catalog = SelectorCatalog3::new(
            vec![ConcreteObject3::new(WALL, "Wall")],
            vec![ObjectFamily3::new(
                "Marker",
                vec![VariantAxis3::directions(
                    "direction",
                    DirectionSet3::Directions,
                )],
                vec![
                    ObjectVariant3::new(MARKER_LEFT, vec!["left"]),
                    ObjectVariant3::new(MARKER_RIGHT, vec!["right"]),
                    ObjectVariant3::new(MARKER_UP, vec!["up"]),
                ],
            )],
            vec![SelectorGroup3::new(
                "blocked",
                vec![
                    ObjectSelector3::variant("Marker", vec![SelectorTag3::value("horizontal")]),
                    ObjectSelector3::object("Wall"),
                ],
            )],
        );

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::group("blocked"))
                .unwrap()
                .alternatives,
            vec![MARKER_LEFT, MARKER_RIGHT, WALL]
        );
    }

    #[test]
    fn direction_variant_selector_matches_single_direction_value() {
        let catalog = selector_catalog();

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Marker",
                    vec![SelectorTag3::value("right")]
                ))
                .unwrap()
                .alternatives,
            vec![MARKER_RIGHT]
        );
    }

    #[test]
    fn direction_set_selector_matches_direction_subset() {
        let catalog = selector_catalog();

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Marker",
                    vec![SelectorTag3::value("horizontal")]
                ))
                .unwrap()
                .alternatives,
            vec![MARKER_LEFT, MARKER_RIGHT, MARKER_FORWARD, MARKER_BACKWARD]
        );
        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Marker",
                    vec![SelectorTag3::value("vertical")]
                ))
                .unwrap()
                .alternatives,
            vec![MARKER_UP, MARKER_DOWN]
        );
    }

    #[test]
    fn explicit_any_selector_matches_all_variants() {
        let catalog = selector_catalog();
        let resolved = catalog
            .resolve(&ObjectSelector3::variant(
                "Marker",
                vec![SelectorTag3::any()],
            ))
            .unwrap();

        assert_eq!(resolved.token, "Marker:*");
        assert_eq!(
            resolved.alternatives,
            vec![
                MARKER_LEFT,
                MARKER_RIGHT,
                MARKER_FORWARD,
                MARKER_BACKWARD,
                MARKER_UP,
                MARKER_DOWN,
            ]
        );
    }

    #[test]
    fn any_selector_fills_all_variant_slots_for_multi_axis_family() {
        let target_a_on = ObjectId(20);
        let target_b_on = ObjectId(21);
        let target_a_off = ObjectId(22);
        let catalog = SelectorCatalog3::new(
            Vec::new(),
            vec![ObjectFamily3::new(
                "Target",
                vec![
                    VariantAxis3::named("kind", vec!["A", "B"]),
                    VariantAxis3::named("state", vec!["on", "off"]),
                ],
                vec![
                    ObjectVariant3::new(target_a_on, vec!["A", "on"]),
                    ObjectVariant3::new(target_b_on, vec!["B", "on"]),
                    ObjectVariant3::new(target_a_off, vec!["A", "off"]),
                ],
            )],
            Vec::new(),
        );

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Target",
                    vec![SelectorTag3::any()]
                ))
                .unwrap()
                .alternatives,
            vec![target_a_on, target_b_on, target_a_off]
        );
        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Target",
                    vec![SelectorTag3::value("A"), SelectorTag3::any()]
                ))
                .unwrap()
                .alternatives,
            vec![target_a_on, target_a_off]
        );
    }

    #[test]
    fn partial_multi_axis_variant_selector_is_rejected() {
        let catalog = SelectorCatalog3::new(
            Vec::new(),
            vec![ObjectFamily3::new(
                "Target",
                vec![
                    VariantAxis3::named("kind", vec!["A", "B"]),
                    VariantAxis3::named("state", vec!["on", "off"]),
                ],
                vec![ObjectVariant3::new(ObjectId(20), vec!["A", "on"])],
            )],
            Vec::new(),
        );

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Target",
                    vec![SelectorTag3::value("A")]
                ))
                .unwrap_err(),
            SelectorError3::PartialVariantSelector {
                family: "Target".to_string(),
                expected: 2,
                actual: 1,
            }
        );
    }

    #[test]
    fn bare_variant_family_selector_is_rejected() {
        let catalog = selector_catalog();

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::object("Marker"))
                .unwrap_err(),
            SelectorError3::BareVariantFamily {
                family: "Marker".to_string(),
            }
        );
    }

    #[test]
    fn frame_set_names_are_not_direction_selector_tags() {
        let catalog = selector_catalog();

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Marker",
                    vec![SelectorTag3::value("canonical")]
                ))
                .unwrap_err(),
            SelectorError3::UnknownVariantTag {
                family: "Marker".to_string(),
                axis: "direction".to_string(),
                tag: "canonical".to_string(),
            }
        );
        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "Marker",
                    vec![SelectorTag3::value("mirrored")]
                ))
                .unwrap_err(),
            SelectorError3::UnknownVariantTag {
                family: "Marker".to_string(),
                axis: "direction".to_string(),
                tag: "mirrored".to_string(),
            }
        );
    }

    #[test]
    fn direction_selector_respects_axis_subset() {
        let catalog = SelectorCatalog3::new(
            Vec::new(),
            vec![ObjectFamily3::new(
                "HorizontalMarker",
                vec![VariantAxis3::directions(
                    "direction",
                    DirectionSet3::Horizontal,
                )],
                vec![
                    ObjectVariant3::new(MARKER_LEFT, vec!["left"]),
                    ObjectVariant3::new(MARKER_FORWARD, vec!["front"]),
                ],
            )],
            Vec::new(),
        );

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "HorizontalMarker",
                    vec![SelectorTag3::value("directions")]
                ))
                .unwrap()
                .alternatives,
            vec![MARKER_LEFT, MARKER_FORWARD]
        );
        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::variant(
                    "HorizontalMarker",
                    vec![SelectorTag3::value("vertical")]
                ))
                .unwrap_err(),
            SelectorError3::UnknownVariantTag {
                family: "HorizontalMarker".to_string(),
                axis: "direction".to_string(),
                tag: "vertical".to_string(),
            }
        );
    }

    #[test]
    fn checked_selector_catalog_rejects_shadowed_selector_names() {
        assert_eq!(
            SelectorCatalog3::checked_new(
                vec![ConcreteObject3::new(PLAYER, "Marker")],
                vec![ObjectFamily3::new(
                    "Marker",
                    vec![VariantAxis3::directions(
                        "direction",
                        DirectionSet3::Directions
                    )],
                    vec![ObjectVariant3::new(MARKER_LEFT, vec!["left"])],
                )],
                Vec::new(),
                Vec::new(),
            )
            .unwrap_err(),
            SelectorCatalogError3::FamilyNameShadowsObject {
                name: "Marker".to_string(),
            }
        );

        assert_eq!(
            SelectorCatalog3::checked_new(
                vec![ConcreteObject3::new(PLAYER, "Player")],
                Vec::new(),
                vec![SelectorGroup3::new(
                    "Player",
                    vec![ObjectSelector3::object("Player")]
                )],
                Vec::new(),
            )
            .unwrap_err(),
            SelectorCatalogError3::GroupNameShadowsSelector {
                name: "Player".to_string(),
            }
        );
    }

    #[test]
    fn recursive_group_selector_is_rejected() {
        let catalog = SelectorCatalog3::new(
            Vec::new(),
            Vec::new(),
            vec![SelectorGroup3::new(
                "loop",
                vec![ObjectSelector3::group("loop")],
            )],
        );

        assert_eq!(
            catalog
                .resolve(&ObjectSelector3::group("loop"))
                .unwrap_err(),
            SelectorError3::RecursiveGroup {
                name: "loop".to_string(),
            }
        );
    }

    #[test]
    fn pattern_template_expands_required_selector_alternatives() {
        let catalog = selector_catalog();
        let template = PatternTemplate3::new(vec![MatchCellTemplate3::new(Offset3::ZERO).require(
            ObjectSelector3::variant("Marker", vec![SelectorTag3::value("horizontal")]),
        )]);

        let patterns = lower_pattern_template(&catalog, &template).unwrap();

        assert_eq!(patterns.len(), 4);
        assert_eq!(
            patterns
                .iter()
                .map(|pattern| pattern.cells[0].require_objects.clone())
                .collect::<Vec<_>>(),
            vec![
                vec![MARKER_LEFT],
                vec![MARKER_RIGHT],
                vec![MARKER_FORWARD],
                vec![MARKER_BACKWARD],
            ]
        );
    }

    #[test]
    fn pattern_template_collects_forbidden_selector_alternatives() {
        let catalog = selector_catalog();
        let template = PatternTemplate3::new(vec![
            MatchCellTemplate3::new(Direction3::RIGHT.offset)
                .forbid(ObjectSelector3::group("solid")),
        ]);

        let patterns = lower_pattern_template(&catalog, &template).unwrap();

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].cells[0].offset, Direction3::RIGHT.offset);
        assert_eq!(patterns[0].cells[0].forbid_objects, vec![PLAYER, BOX, WALL]);
    }

    #[test]
    fn pattern_template_expands_required_selectors_as_cartesian_product() {
        let catalog = selector_catalog();
        let template = PatternTemplate3::new(vec![
            MatchCellTemplate3::new(Offset3::ZERO).require(ObjectSelector3::variant(
                "Marker",
                vec![SelectorTag3::value("horizontal")],
            )),
            MatchCellTemplate3::new(Direction3::UP.offset).require(ObjectSelector3::variant(
                "Marker",
                vec![SelectorTag3::value("vertical")],
            )),
        ]);

        let patterns = lower_pattern_template(&catalog, &template).unwrap();

        assert_eq!(patterns.len(), 8);
        assert_eq!(patterns[0].cells[0].require_objects, vec![MARKER_LEFT]);
        assert_eq!(patterns[0].cells[1].require_objects, vec![MARKER_UP]);
        assert_eq!(patterns[1].cells[0].require_objects, vec![MARKER_LEFT]);
        assert_eq!(patterns[1].cells[1].require_objects, vec![MARKER_DOWN]);
    }

    #[test]
    fn repeated_required_selector_token_preserves_assignment() {
        let catalog = selector_catalog();
        let template = PatternTemplate3::new(vec![
            MatchCellTemplate3::new(Offset3::ZERO).require(ObjectSelector3::variant(
                "Marker",
                vec![SelectorTag3::any()],
            )),
            MatchCellTemplate3::new(Direction3::RIGHT.offset).require(ObjectSelector3::variant(
                "Marker",
                vec![SelectorTag3::any()],
            )),
        ]);

        let patterns = lower_pattern_template(&catalog, &template).unwrap();

        assert_eq!(patterns.len(), 6);
        assert!(patterns.iter().all(|pattern| {
            pattern.cells[0].require_objects[0] == pattern.cells[1].require_objects[0]
        }));
        assert_eq!(
            patterns
                .iter()
                .map(|pattern| pattern.cells[0].require_objects[0])
                .collect::<Vec<_>>(),
            vec![
                MARKER_LEFT,
                MARKER_RIGHT,
                MARKER_FORWARD,
                MARKER_BACKWARD,
                MARKER_UP,
                MARKER_DOWN,
            ]
        );
    }

    #[test]
    fn pattern_template_reports_selector_errors() {
        let catalog = selector_catalog();
        let template = PatternTemplate3::new(vec![MatchCellTemplate3::new(Offset3::ZERO).require(
            ObjectSelector3::variant("Marker", vec![SelectorTag3::value("frames")]),
        )]);

        assert_eq!(
            lower_pattern_template(&catalog, &template).unwrap_err(),
            PatternLoweringError3::Selector(SelectorError3::UnknownVariantTag {
                family: "Marker".to_string(),
                axis: "direction".to_string(),
                tag: "frames".to_string(),
            })
        );
    }

    #[test]
    fn rule_template_lowers_selector_assignments_into_move_writes() {
        let catalog = selector_catalog();
        let pattern = PatternTemplate3::new(vec![MatchCellTemplate3::new(Offset3::ZERO).require(
            ObjectSelector3::variant("Marker", vec![SelectorTag3::value("horizontal")]),
        )]);
        let rule = RuleTemplate3::once(
            pattern,
            vec![WriteOpTemplate3::Move {
                from_offset: Offset3::ZERO,
                to_offset: Direction3::RIGHT.offset,
                object: ObjectSelector3::variant("Marker", vec![SelectorTag3::value("horizontal")]),
            }],
        )
        .with_id(RuleId3(42))
        .when_input(INPUT_RIGHT);

        let rules = lower_rule_template(&catalog, &rule).unwrap();

        assert_eq!(rules.len(), 4);
        assert_eq!(rules[0].id, RuleId3(42));
        assert_eq!(rules[0].guards, vec![Guard3::InputIs(INPUT_RIGHT)]);
        assert_eq!(rules[0].application, RuleApplication3::Once);
        for rule in &rules {
            let required = rule.pattern.cells[0].require_objects[0];
            assert_eq!(
                rule.writes,
                vec![WriteOp3::Move {
                    component: 0,
                    from_offset: Offset3::ZERO,
                    to_offset: Direction3::RIGHT.offset,
                    object: required,
                }]
            );
        }
    }

    #[test]
    fn rule_template_allows_unassigned_singleton_write_selector() {
        let catalog = selector_catalog();
        let pattern = PatternTemplate3::new(vec![
            MatchCellTemplate3::new(Offset3::ZERO).require(ObjectSelector3::object("Player")),
        ]);
        let rule = RuleTemplate3::once(
            pattern,
            vec![WriteOpTemplate3::Add {
                offset: Direction3::UP.offset,
                object: ObjectSelector3::object("Wall"),
            }],
        );

        let rules = lower_rule_template(&catalog, &rule).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(
            rules[0].writes,
            vec![WriteOp3::Add {
                component: 0,
                offset: Direction3::UP.offset,
                object: WALL,
            }]
        );
    }

    #[test]
    fn rule_template_rejects_ambiguous_unassigned_write_selector() {
        let catalog = selector_catalog();
        let pattern = PatternTemplate3::new(vec![
            MatchCellTemplate3::new(Offset3::ZERO).require(ObjectSelector3::object("Player")),
        ]);
        let rule = RuleTemplate3::once(
            pattern,
            vec![WriteOpTemplate3::Add {
                offset: Direction3::UP.offset,
                object: ObjectSelector3::variant("Marker", vec![SelectorTag3::value("horizontal")]),
            }],
        );

        assert_eq!(
            lower_rule_template(&catalog, &rule).unwrap_err(),
            RuleLoweringError3::AmbiguousWriteSelector {
                token: "Marker:horizontal".to_string(),
                alternatives: vec![MARKER_LEFT, MARKER_RIGHT, MARKER_FORWARD, MARKER_BACKWARD],
            }
        );
    }

    #[test]
    fn rule_template_lowers_replace_with_bound_remove_and_singleton_add() {
        let catalog = selector_catalog();
        let pattern = PatternTemplate3::new(vec![MatchCellTemplate3::new(Offset3::ZERO).require(
            ObjectSelector3::variant("Marker", vec![SelectorTag3::value("vertical")]),
        )]);
        let rule = RuleTemplate3::once(
            pattern,
            vec![WriteOpTemplate3::Replace {
                offset: Offset3::ZERO,
                remove: ObjectSelector3::variant("Marker", vec![SelectorTag3::value("vertical")]),
                add: ObjectSelector3::object("Wall"),
            }],
        );

        let rules = lower_rule_template(&catalog, &rule).unwrap();

        assert_eq!(rules.len(), 2);
        assert_eq!(
            rules[0].writes,
            vec![WriteOp3::Replace {
                component: 0,
                offset: Offset3::ZERO,
                remove: MARKER_UP,
                add: WALL,
            }]
        );
        assert_eq!(
            rules[1].writes,
            vec![WriteOp3::Replace {
                component: 0,
                offset: Offset3::ZERO,
                remove: MARKER_DOWN,
                add: WALL,
            }]
        );
    }

    #[test]
    fn dense_pattern_lowers_columns_rows_and_slices_through_frame() {
        let dense = DensePattern3::new(vec![
            DenseSlice3::new(vec![
                DenseRow3::new(vec![
                    DenseCell3::require(ObjectSelector3::object("Player")),
                    DenseCell3::require(ObjectSelector3::object("Box")),
                ]),
                DenseRow3::new(vec![
                    DenseCell3::empty(),
                    DenseCell3::forbid(ObjectSelector3::group("solid")),
                ]),
            ]),
            DenseSlice3::new(vec![DenseRow3::new(vec![DenseCell3::require(
                ObjectSelector3::object("Wall"),
            )])]),
        ]);

        let template = lower_dense_pattern(Frame3::DEFAULT, &dense);

        assert_eq!(
            template
                .cells
                .iter()
                .map(|cell| cell.offset)
                .collect::<Vec<_>>(),
            vec![
                Offset3::ZERO,
                Direction3::RIGHT.offset,
                Direction3::RIGHT.offset.add(Direction3::BACKWARD.offset),
                Direction3::DOWN.offset,
            ]
        );
        assert_eq!(
            template.cells[0].require,
            vec![ObjectSelector3::object("Player")]
        );
        assert_eq!(
            template.cells[2].forbid,
            vec![ObjectSelector3::group("solid")]
        );
    }

    #[test]
    fn dense_pattern_uses_frame_orientation_for_world_offsets() {
        let frame = Frame3::canonical(Direction3::FORWARD, Direction3::UP).unwrap();
        let dense = DensePattern3::new(vec![DenseSlice3::new(vec![
            DenseRow3::new(vec![
                DenseCell3::require(ObjectSelector3::object("Player")),
                DenseCell3::require(ObjectSelector3::object("Box")),
            ]),
            DenseRow3::new(vec![DenseCell3::require(ObjectSelector3::object("Wall"))]),
        ])]);

        let template = lower_dense_pattern(frame, &dense);

        assert_eq!(
            template
                .cells
                .iter()
                .map(|cell| cell.offset)
                .collect::<Vec<_>>(),
            vec![
                Offset3::ZERO,
                Direction3::FORWARD.offset,
                Direction3::UP.offset,
            ]
        );
    }

    #[test]
    fn dense_pattern_connects_to_selector_pattern_lowering() {
        let catalog = selector_catalog();
        let dense = DensePattern3::new(vec![DenseSlice3::new(vec![DenseRow3::new(vec![
            DenseCell3::require(ObjectSelector3::variant(
                "Marker",
                vec![SelectorTag3::value("horizontal")],
            )),
            DenseCell3::forbid(ObjectSelector3::group("solid")),
        ])])]);

        let patterns = lower_dense_pattern_to_patterns(&catalog, Frame3::DEFAULT, &dense).unwrap();

        assert_eq!(patterns.len(), 4);
        assert_eq!(patterns[0].cells[0].offset, Offset3::ZERO);
        assert_eq!(patterns[0].cells[0].require_objects, vec![MARKER_LEFT]);
        assert_eq!(patterns[0].cells[1].offset, Direction3::RIGHT.offset);
        assert_eq!(patterns[0].cells[1].forbid_objects, vec![PLAYER, BOX, WALL]);
    }

    #[test]
    fn dense_pattern_set_expands_all_frames_before_selector_lowering() {
        let catalog = selector_catalog();
        let dense = DensePattern3::new(vec![DenseSlice3::new(vec![DenseRow3::new(vec![
            DenseCell3::require(ObjectSelector3::object("Player")),
            DenseCell3::require(ObjectSelector3::object("Box")),
        ])])]);

        let patterns =
            lower_dense_pattern_set_to_patterns(&catalog, FrameSet3::Canonical, &dense).unwrap();

        assert_eq!(patterns.len(), 24);
        assert!(patterns.iter().any(|pattern| {
            pattern.cells[0].offset == Offset3::ZERO
                && pattern.cells[1].offset == Direction3::RIGHT.offset
        }));
        assert!(patterns.iter().any(|pattern| {
            pattern.cells[0].offset == Offset3::ZERO
                && pattern.cells[1].offset == Direction3::UP.offset
        }));
    }

    #[test]
    fn line_rule_template_expands_direction_set_sugar_to_concrete_rules() {
        let catalog = selector_catalog();
        let rule = LineRuleTemplate3::once(
            LineOrientation3::DirectionSet(DirectionSet3::Horizontal),
            LinePatternTemplate3::new(vec![
                LineMatchCellTemplate3::new(0).require(ObjectSelector3::object("Player")),
                LineMatchCellTemplate3::new(1).forbid(ObjectSelector3::group("solid")),
            ]),
            vec![LineWriteOpTemplate3::Move {
                from_step: 0,
                to_step: 1,
                object: ObjectSelector3::object("Player"),
            }],
        );

        let rules = lower_line_rule_template(&catalog, &rule).unwrap();

        assert_eq!(rules.len(), 4);
        assert_eq!(
            rules
                .iter()
                .map(|rule| rule.pattern.cells[1].offset)
                .collect::<Vec<_>>(),
            vec![
                Direction3::LEFT.offset,
                Direction3::RIGHT.offset,
                Direction3::FORWARD.offset,
                Direction3::BACKWARD.offset,
            ]
        );
        assert_eq!(
            rules[1].writes,
            vec![WriteOp3::Move {
                component: 0,
                from_offset: Offset3::ZERO,
                to_offset: Direction3::RIGHT.offset,
                object: PLAYER,
            }]
        );
    }

    #[test]
    fn dense_rule_template_transforms_local_writes_through_frame() {
        let catalog = selector_catalog();
        let frame = Frame3::canonical(Direction3::FORWARD, Direction3::UP).unwrap();
        let rule = DenseRuleTemplate3::once(
            FrameOrientation3::Frame(frame),
            DensePattern3::new(vec![DenseSlice3::new(vec![DenseRow3::new(vec![
                DenseCell3::require(ObjectSelector3::object("Player")),
                DenseCell3::require(ObjectSelector3::object("Box")),
            ])])]),
            vec![LocalWriteOpTemplate3::Move {
                from_offset: Offset3::new(1, 0, 0),
                to_offset: Offset3::new(2, 0, 0),
                object: ObjectSelector3::object("Box"),
            }],
        );

        let rules = lower_dense_rule_template(&catalog, &rule).unwrap();

        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].pattern.cells[1].offset, Direction3::FORWARD.offset);
        assert_eq!(
            rules[0].writes,
            vec![WriteOp3::Move {
                component: 0,
                from_offset: Direction3::FORWARD.offset,
                to_offset: Direction3::FORWARD.offset.scale(2),
                object: BOX,
            }]
        );
    }

    #[test]
    fn dense_rule_template_expands_frame_set_sugar_to_concrete_rules() {
        let catalog = selector_catalog();
        let rule = DenseRuleTemplate3::once(
            FrameOrientation3::FrameSet(FrameSet3::Canonical),
            DensePattern3::new(vec![DenseSlice3::new(vec![DenseRow3::new(vec![
                DenseCell3::require(ObjectSelector3::object("Player")),
                DenseCell3::require(ObjectSelector3::object("Box")),
            ])])]),
            vec![LocalWriteOpTemplate3::Move {
                from_offset: Offset3::ZERO,
                to_offset: Offset3::new(1, 0, 0),
                object: ObjectSelector3::object("Player"),
            }],
        );

        let rules = lower_dense_rule_template(&catalog, &rule).unwrap();

        assert_eq!(rules.len(), 24);
        assert!(rules.iter().any(|rule| {
            rule.pattern.cells[1].offset == Direction3::RIGHT.offset
                && rule.writes
                    == vec![WriteOp3::Move {
                        component: 0,
                        from_offset: Offset3::ZERO,
                        to_offset: Direction3::RIGHT.offset,
                        object: PLAYER,
                    }]
        }));
        assert!(rules.iter().any(|rule| {
            rule.pattern.cells[1].offset == Direction3::UP.offset
                && rule.writes
                    == vec![WriteOp3::Move {
                        component: 0,
                        from_offset: Offset3::ZERO,
                        to_offset: Direction3::UP.offset,
                        object: PLAYER,
                    }]
        }));
    }
}
