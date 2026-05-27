use crate::{
    Direction3, DirectionSet3, Frame3, FrameSet3, Guard3, MatchCell3, ObjectId, Offset3, Pattern3,
    Rule3, RuleApplication3, RuleId3, WriteOp3,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorCatalog3 {
    pub objects: Vec<ConcreteObject3>,
    pub families: Vec<ObjectFamily3>,
    pub groups: Vec<SelectorGroup3>,
}

impl SelectorCatalog3 {
    pub fn new(
        objects: Vec<ConcreteObject3>,
        families: Vec<ObjectFamily3>,
        groups: Vec<SelectorGroup3>,
    ) -> Self {
        Self {
            objects,
            families,
            groups,
        }
    }

    pub fn checked_new(
        objects: Vec<ConcreteObject3>,
        families: Vec<ObjectFamily3>,
        groups: Vec<SelectorGroup3>,
    ) -> Result<Self, SelectorCatalogError3> {
        validate_catalog_names(&objects, &families, &groups)?;
        Ok(Self::new(objects, families, groups))
    }

    pub fn resolve(&self, selector: &ObjectSelector3) -> Result<ResolvedSelector3, SelectorError3> {
        let alternatives = match selector {
            ObjectSelector3::Object(name) => self.resolve_object(name)?,
            ObjectSelector3::Group(name) => self.resolve_group(name, &mut Vec::new())?,
            ObjectSelector3::Labeled { selector, .. } => self.resolve(selector)?.alternatives,
            ObjectSelector3::Variant { family, tags } => self.resolve_variant(family, tags)?,
        };
        let token = selector.token();
        if alternatives.is_empty() {
            return Err(SelectorError3::SelectorMatchedNoObjects { token });
        }
        Ok(ResolvedSelector3 {
            token,
            alternatives,
            transform: None,
            scratch: Vec::new(),
        })
    }

    fn resolve_object(&self, name: &str) -> Result<Vec<ObjectId>, SelectorError3> {
        if let Some(object) = self.objects.iter().find(|object| object.name == name) {
            return Ok(vec![object.id]);
        }
        if self.families.iter().any(|family| family.name == name) {
            return Err(SelectorError3::BareVariantFamily {
                family: name.to_string(),
            });
        }
        Err(SelectorError3::UnknownObject {
            name: name.to_string(),
        })
    }

    fn resolve_group(
        &self,
        name: &str,
        stack: &mut Vec<String>,
    ) -> Result<Vec<ObjectId>, SelectorError3> {
        if stack.iter().any(|entry| entry == name) {
            return Err(SelectorError3::RecursiveGroup {
                name: name.to_string(),
            });
        }
        let Some(group) = self.groups.iter().find(|group| group.name == name) else {
            return Err(SelectorError3::UnknownGroup {
                name: name.to_string(),
            });
        };

        stack.push(name.to_string());
        let mut objects = Vec::new();
        for selector in &group.selectors {
            let alternatives = match selector {
                ObjectSelector3::Object(name) => self.resolve_object(name)?,
                ObjectSelector3::Group(name) => self.resolve_group(name, stack)?,
                ObjectSelector3::Labeled { selector, .. } => self.resolve(selector)?.alternatives,
                ObjectSelector3::Variant { family, tags } => self.resolve_variant(family, tags)?,
            };
            for object in alternatives {
                push_unique_object(&mut objects, object);
            }
        }
        stack.pop();
        Ok(objects)
    }

    fn resolve_variant(
        &self,
        family_name: &str,
        tags: &[SelectorTag3],
    ) -> Result<Vec<ObjectId>, SelectorError3> {
        let Some(family) = self
            .families
            .iter()
            .find(|family| family.name == family_name)
        else {
            return Err(SelectorError3::UnknownFamily {
                family: family_name.to_string(),
            });
        };
        let tags = expanded_tags(family, tags)?;

        for (axis, tag) in family.axes.iter().zip(&tags) {
            if let SelectorTag3::Value(value) = tag {
                if axis.allowed_values(value).is_none() {
                    return Err(SelectorError3::UnknownVariantTag {
                        family: family.name.clone(),
                        axis: axis.name.clone(),
                        tag: value.clone(),
                    });
                }
            }
        }

        let mut objects = Vec::new();
        for variant in &family.variants {
            if variant.values.len() != family.axes.len() {
                continue;
            }
            if family
                .axes
                .iter()
                .zip(&tags)
                .zip(&variant.values)
                .all(|((axis, tag), variant_value)| axis.tag_accepts_value(tag, variant_value))
            {
                push_unique_object(&mut objects, variant.id);
            }
        }
        Ok(objects)
    }
}

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

    pub fn when_input(mut self, input: crate::InputId3) -> Self {
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

    pub fn when_input(mut self, input: crate::InputId3) -> Self {
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
                    require: cell.require.clone(),
                    forbid: cell.forbid.clone(),
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

    pub fn when_input(mut self, input: crate::InputId3) -> Self {
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
    object: ObjectId,
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
        }
    }

    let mut partials = vec![(cell, assignments)];
    for selector in &template.require {
        let resolved = catalog.resolve(selector)?;
        let mut next = Vec::new();
        for (cell, assignments) in partials {
            if let Some(assigned) = assignments
                .iter()
                .find(|assignment| assignment.token == resolved.token)
                .map(|assignment| assignment.object)
            {
                if resolved.alternatives.contains(&assigned) {
                    let mut cell = cell;
                    push_unique_object(&mut cell.require_objects, assigned);
                    next.push((cell, assignments));
                }
                continue;
            }

            for object in &resolved.alternatives {
                let mut cell = cell.clone();
                let mut assignments = assignments.clone();
                push_unique_object(&mut cell.require_objects, *object);
                assignments.push(SelectorAssignment3 {
                    token: resolved.token.clone(),
                    object: *object,
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

fn lower_write_templates(
    catalog: &SelectorCatalog3,
    assignments: &[SelectorAssignment3],
    templates: &[WriteOpTemplate3],
) -> Result<Vec<WriteOp3>, RuleLoweringError3> {
    templates
        .iter()
        .map(|template| match template {
            WriteOpTemplate3::Add { offset, object } => Ok(WriteOp3::Add {
                offset: *offset,
                object: write_object(catalog, assignments, object)?,
            }),
            WriteOpTemplate3::Remove { offset, object } => Ok(WriteOp3::Remove {
                offset: *offset,
                object: write_object(catalog, assignments, object)?,
            }),
            WriteOpTemplate3::Replace {
                offset,
                remove,
                add,
            } => Ok(WriteOp3::Replace {
                offset: *offset,
                remove: write_object(catalog, assignments, remove)?,
                add: write_object(catalog, assignments, add)?,
            }),
            WriteOpTemplate3::Move {
                from_offset,
                to_offset,
                object,
            } => Ok(WriteOp3::Move {
                from_offset: *from_offset,
                to_offset: *to_offset,
                object: write_object(catalog, assignments, object)?,
            }),
        })
        .collect()
}

fn write_object(
    catalog: &SelectorCatalog3,
    assignments: &[SelectorAssignment3],
    selector: &ObjectSelector3,
) -> Result<ObjectId, RuleLoweringError3> {
    let token = selector.token();
    if let Some(object) = assignments
        .iter()
        .find(|assignment| assignment.token == token)
        .map(|assignment| assignment.object)
    {
        return Ok(object);
    }

    if selector.has_occurrence_label() {
        return Err(RuleLoweringError3::UnboundSelectorOccurrenceLabel { token });
    }

    let resolved = catalog.resolve(selector)?;
    if resolved.alternatives.len() == 1 {
        return Ok(resolved.alternatives[0]);
    }

    Err(RuleLoweringError3::AmbiguousWriteSelector {
        token: resolved.token,
        alternatives: resolved.alternatives,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedSelector3 {
    pub token: String,
    pub alternatives: Vec<ObjectId>,
    pub transform: Option<SelectorTransform3>,
    pub scratch: Vec<SelectorScratch3>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorTransform3 {
    pub source_token: String,
    pub mapped_objects: Vec<(ObjectId, ObjectId)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorScratch3 {
    pub name: String,
    pub value: Option<String>,
    pub negated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConcreteObject3 {
    pub id: ObjectId,
    pub name: String,
}

impl ConcreteObject3 {
    pub fn new(id: ObjectId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorGroup3 {
    pub name: String,
    pub selectors: Vec<ObjectSelector3>,
}

impl SelectorGroup3 {
    pub fn new(name: impl Into<String>, selectors: Vec<ObjectSelector3>) -> Self {
        Self {
            name: name.into(),
            selectors,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectFamily3 {
    pub name: String,
    pub axes: Vec<VariantAxis3>,
    pub variants: Vec<ObjectVariant3>,
}

impl ObjectFamily3 {
    pub fn new(
        name: impl Into<String>,
        axes: Vec<VariantAxis3>,
        variants: Vec<ObjectVariant3>,
    ) -> Self {
        Self {
            name: name.into(),
            axes,
            variants,
        }
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

    fn tag_accepts_value(&self, tag: &SelectorTag3, value: &str) -> bool {
        match tag {
            SelectorTag3::Any => self.contains_value(value),
            SelectorTag3::Value(tag) => self
                .allowed_values(tag)
                .is_some_and(|values| values.iter().any(|allowed| allowed == value)),
        }
    }

    fn contains_value(&self, value: &str) -> bool {
        self.allowed_values(value)
            .is_some_and(|values| values.iter().any(|allowed| allowed == value))
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
pub struct ObjectVariant3 {
    pub id: ObjectId,
    pub values: Vec<String>,
}

impl ObjectVariant3 {
    pub fn new(id: ObjectId, values: Vec<impl Into<String>>) -> Self {
        Self {
            id,
            values: values.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectSelector3 {
    Object(String),
    Group(String),
    Labeled {
        token: String,
        selector: Box<ObjectSelector3>,
    },
    Variant {
        family: String,
        tags: Vec<SelectorTag3>,
    },
}

impl ObjectSelector3 {
    pub fn object(name: impl Into<String>) -> Self {
        Self::Object(name.into())
    }

    pub fn group(name: impl Into<String>) -> Self {
        Self::Group(name.into())
    }

    pub fn labeled(token: impl Into<String>, selector: ObjectSelector3) -> Self {
        Self::Labeled {
            token: token.into(),
            selector: Box::new(selector),
        }
    }

    pub fn variant(family: impl Into<String>, tags: Vec<SelectorTag3>) -> Self {
        Self::Variant {
            family: family.into(),
            tags,
        }
    }

    pub fn token(&self) -> String {
        match self {
            Self::Object(name) | Self::Group(name) => name.clone(),
            Self::Labeled { token, .. } => token.clone(),
            Self::Variant { family, tags } => {
                let mut token = family.clone();
                for tag in tags {
                    token.push(':');
                    token.push_str(&tag.token());
                }
                token
            }
        }
    }

    pub fn has_occurrence_label(&self) -> bool {
        matches!(self, Self::Labeled { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectorTag3 {
    Value(String),
    Any,
}

impl SelectorTag3 {
    pub fn value(value: impl Into<String>) -> Self {
        Self::Value(value.into())
    }

    pub const fn any() -> Self {
        Self::Any
    }

    pub fn token(&self) -> String {
        match self {
            Self::Value(value) => value.clone(),
            Self::Any => "*".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectorCatalogError3 {
    DuplicateObjectName { name: String },
    DuplicateFamilyName { name: String },
    DuplicateGroupName { name: String },
    ObjectNameShadowsFamily { name: String },
    FamilyNameShadowsObject { name: String },
    GroupNameShadowsSelector { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectorError3 {
    UnknownObject {
        name: String,
    },
    UnknownGroup {
        name: String,
    },
    UnknownFamily {
        family: String,
    },
    BareVariantFamily {
        family: String,
    },
    WrongVariantArity {
        family: String,
        expected: usize,
        actual: usize,
    },
    PartialVariantSelector {
        family: String,
        expected: usize,
        actual: usize,
    },
    UnknownVariantTag {
        family: String,
        axis: String,
        tag: String,
    },
    RecursiveGroup {
        name: String,
    },
    SelectorMatchedNoObjects {
        token: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternLoweringError3 {
    Selector(SelectorError3),
    DuplicateSelectorOccurrenceLabel { token: String },
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

fn validate_catalog_names(
    objects: &[ConcreteObject3],
    families: &[ObjectFamily3],
    groups: &[SelectorGroup3],
) -> Result<(), SelectorCatalogError3> {
    let mut object_names = Vec::<String>::new();
    for object in objects {
        if object_names.contains(&object.name) {
            return Err(SelectorCatalogError3::DuplicateObjectName {
                name: object.name.clone(),
            });
        }
        object_names.push(object.name.clone());
    }

    let mut family_names = Vec::<String>::new();
    for family in families {
        if family_names.contains(&family.name) {
            return Err(SelectorCatalogError3::DuplicateFamilyName {
                name: family.name.clone(),
            });
        }
        if object_names.contains(&family.name) {
            return Err(SelectorCatalogError3::FamilyNameShadowsObject {
                name: family.name.clone(),
            });
        }
        family_names.push(family.name.clone());
    }

    for object_name in &object_names {
        if family_names.contains(object_name) {
            return Err(SelectorCatalogError3::ObjectNameShadowsFamily {
                name: object_name.clone(),
            });
        }
    }

    let mut group_names = Vec::<String>::new();
    for group in groups {
        if group_names.contains(&group.name) {
            return Err(SelectorCatalogError3::DuplicateGroupName {
                name: group.name.clone(),
            });
        }
        if object_names.contains(&group.name)
            || family_names.contains(&group.name)
            || group_names.contains(&group.name)
        {
            return Err(SelectorCatalogError3::GroupNameShadowsSelector {
                name: group.name.clone(),
            });
        }
        group_names.push(group.name.clone());
    }

    Ok(())
}

fn expanded_tags(
    family: &ObjectFamily3,
    tags: &[SelectorTag3],
) -> Result<Vec<SelectorTag3>, SelectorError3> {
    if tags.is_empty() {
        return Err(SelectorError3::BareVariantFamily {
            family: family.name.clone(),
        });
    }
    if tags.len() > family.axes.len() {
        return Err(SelectorError3::WrongVariantArity {
            family: family.name.clone(),
            expected: family.axes.len(),
            actual: tags.len(),
        });
    }
    if tags.len() == 1 && tags[0] == SelectorTag3::Any {
        return Ok(vec![SelectorTag3::Any; family.axes.len()]);
    }
    if tags.len() < family.axes.len() {
        return Err(SelectorError3::PartialVariantSelector {
            family: family.name.clone(),
            expected: family.axes.len(),
            actual: tags.len(),
        });
    }
    Ok(tags.to_vec())
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
