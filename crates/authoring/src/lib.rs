use std::ops::Range;

mod sprite_order_surface;

pub use sprite_order_surface::{
    SpriteOrderItemSurface, SpriteOrderSurface, SpriteOrderSurfaceError, parse_sprite_order_surface,
};

pub trait VariantAxisSpec {
    fn name(&self) -> &str;
    fn allowed_values(&self, tag: &str) -> Option<Vec<String>>;

    fn tag_accepts_value(&self, tag: &SelectorTag, value: &str) -> bool {
        match tag {
            SelectorTag::Any => self
                .allowed_values(value)
                .is_some_and(|values| values.iter().any(|allowed| allowed == value)),
            SelectorTag::Value(tag) => self
                .allowed_values(tag)
                .is_some_and(|values| values.iter().any(|allowed| allowed == value)),
        }
    }
}

/// A direction relative to the spatial orientation of the rule currently
/// being lowered. It is language semantics, not a 2D or 3D catalog alias.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelativeDirection {
    Forward,
    Backward,
    Left,
    Right,
}

pub fn relative_direction(value: &str) -> Option<RelativeDirection> {
    match value {
        ">" => Some(RelativeDirection::Forward),
        "<" => Some(RelativeDirection::Backward),
        "^" => Some(RelativeDirection::Left),
        "v" => Some(RelativeDirection::Right),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantAxis {
    pub name: String,
    values: Vec<String>,
    aliases: Vec<(String, Vec<String>)>,
}

impl VariantAxis {
    pub fn named(name: impl Into<String>, values: Vec<impl Into<String>>) -> Self {
        let values = values.into_iter().map(Into::into).collect::<Vec<_>>();
        let aliases = values
            .iter()
            .map(|value| (value.clone(), vec![value.clone()]))
            .collect();
        Self {
            name: name.into(),
            values,
            aliases,
        }
    }

    pub fn with_aliases(
        name: impl Into<String>,
        values: Vec<String>,
        mut aliases: Vec<(String, Vec<String>)>,
    ) -> Self {
        for value in &values {
            if !aliases.iter().any(|(alias, _)| alias == value) {
                aliases.push((value.clone(), vec![value.clone()]));
            }
        }
        Self {
            name: name.into(),
            values,
            aliases,
        }
    }
}

impl VariantAxisSpec for VariantAxis {
    fn name(&self) -> &str {
        &self.name
    }

    fn allowed_values(&self, tag: &str) -> Option<Vec<String>> {
        let requested = self
            .aliases
            .iter()
            .find_map(|(alias, values)| (alias == tag).then_some(values))?;
        let values = requested
            .iter()
            .filter(|value| self.values.contains(value))
            .cloned()
            .collect::<Vec<_>>();
        (!values.is_empty()).then_some(values)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorCatalog<ObjectId, LayerId, Axis, Mark> {
    pub objects: Vec<ConcreteObject<ObjectId>>,
    pub families: Vec<ObjectFamily<ObjectId, Axis>>,
    pub groups: Vec<SelectorGroup<ObjectSelector<Mark>>>,
    object_layers: Vec<(ObjectId, LayerId)>,
}

pub type ModelCatalog =
    SelectorCatalog<puzzle_kernel::ObjectId, puzzle_kernel::LayerId, VariantAxis, SelectorMark>;

impl<ObjectId, LayerId, Axis, Mark> SelectorCatalog<ObjectId, LayerId, Axis, Mark>
where
    ObjectId: Copy + Eq,
    LayerId: Copy + Default,
    Axis: VariantAxisSpec,
    Mark: Clone,
{
    pub fn new(
        objects: Vec<ConcreteObject<ObjectId>>,
        families: Vec<ObjectFamily<ObjectId, Axis>>,
        groups: Vec<SelectorGroup<ObjectSelector<Mark>>>,
    ) -> Self {
        let object_layers = default_object_layers(&objects, &families);
        Self {
            objects,
            families,
            groups,
            object_layers,
        }
    }

    pub fn checked_new(
        objects: Vec<ConcreteObject<ObjectId>>,
        families: Vec<ObjectFamily<ObjectId, Axis>>,
        groups: Vec<SelectorGroup<ObjectSelector<Mark>>>,
        object_layers: Vec<(ObjectId, LayerId)>,
    ) -> Result<Self, SelectorCatalogError> {
        validate_catalog_names(&objects, &families, &groups)?;
        Ok(Self {
            objects,
            families,
            groups,
            object_layers,
        })
    }

    pub fn object_layer(&self, object: ObjectId) -> Option<LayerId> {
        self.object_layers
            .iter()
            .find(|(candidate, _)| *candidate == object)
            .map(|(_, layer)| *layer)
    }

    pub fn resolve(
        &self,
        selector: &ObjectSelector<Mark>,
    ) -> Result<ResolvedSelector<ObjectId, Mark>, SelectorError> {
        let (alternatives, mark) = match selector {
            ObjectSelector::Any => (self.resolve_any(), Vec::new()),
            ObjectSelector::Object(name) => (self.resolve_object(name)?, Vec::new()),
            ObjectSelector::Group(name) => (self.resolve_group(name, &mut Vec::new())?, Vec::new()),
            ObjectSelector::Labeled { selector, .. } => {
                let resolved = self.resolve(selector)?;
                (resolved.alternatives, resolved.mark)
            }
            ObjectSelector::Variant { family, tags } => {
                (self.resolve_variant(family, tags)?, Vec::new())
            }
            ObjectSelector::WithMark { selector, mark } => {
                let resolved = self.resolve(selector)?;
                let mut combined = resolved.mark;
                combined.extend(mark.iter().cloned());
                (resolved.alternatives, combined)
            }
        };
        let token = selector.token();
        if alternatives.is_empty() {
            return Err(SelectorError::SelectorMatchedNoObjects { token });
        }
        Ok(ResolvedSelector {
            token,
            alternatives,
            mark,
        })
    }

    fn resolve_any(&self) -> Vec<ObjectId> {
        let mut objects = Vec::new();
        for object in &self.objects {
            push_unique(&mut objects, object.id);
        }
        for family in &self.families {
            for variant in &family.variants {
                push_unique(&mut objects, variant.id);
            }
        }
        objects
    }

    fn resolve_object(&self, name: &str) -> Result<Vec<ObjectId>, SelectorError> {
        if let Some(object) = self.objects.iter().find(|object| object.name == name) {
            return Ok(vec![object.id]);
        }
        if self.families.iter().any(|family| family.name == name) {
            return Err(SelectorError::BareVariantFamily {
                family: name.to_string(),
            });
        }
        Err(SelectorError::UnknownObject {
            name: name.to_string(),
        })
    }

    fn resolve_group(
        &self,
        name: &str,
        stack: &mut Vec<String>,
    ) -> Result<Vec<ObjectId>, SelectorError> {
        if stack.iter().any(|entry| entry == name) {
            return Err(SelectorError::RecursiveGroup {
                name: name.to_string(),
            });
        }
        let Some(group) = self.groups.iter().find(|group| group.name == name) else {
            return Err(SelectorError::UnknownGroup {
                name: name.to_string(),
            });
        };

        stack.push(name.to_string());
        let mut objects = Vec::new();
        for selector in &group.selectors {
            let alternatives = match selector {
                ObjectSelector::Any => self.resolve_any(),
                ObjectSelector::Object(name) => self.resolve_object(name)?,
                ObjectSelector::Group(name) => self.resolve_group(name, stack)?,
                ObjectSelector::Labeled { selector, .. } => self.resolve(selector)?.alternatives,
                ObjectSelector::Variant { family, tags } => self.resolve_variant(family, tags)?,
                ObjectSelector::WithMark { selector, .. } => self.resolve(selector)?.alternatives,
            };
            for object in alternatives {
                push_unique(&mut objects, object);
            }
        }
        stack.pop();
        Ok(objects)
    }

    fn resolve_variant(
        &self,
        family_name: &str,
        tags: &[SelectorTag],
    ) -> Result<Vec<ObjectId>, SelectorError> {
        let Some(family) = self
            .families
            .iter()
            .find(|family| family.name == family_name)
        else {
            return Err(SelectorError::UnknownFamily {
                family: family_name.to_string(),
            });
        };
        let tags = expanded_tags(family, tags)?;

        for (axis, tag) in family.axes.iter().zip(&tags) {
            if let SelectorTag::Value(value) = tag
                && axis.allowed_values(value).is_none()
            {
                return Err(SelectorError::UnknownVariantTag {
                    family: family.name.clone(),
                    axis: axis.name().to_string(),
                    tag: value.clone(),
                });
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
                push_unique(&mut objects, variant.id);
            }
        }
        Ok(objects)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedSelector<ObjectId, Mark> {
    pub token: String,
    pub alternatives: Vec<ObjectId>,
    pub mark: Vec<Mark>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorMark {
    pub name: String,
    pub value: Option<String>,
    pub negated: bool,
}

/// Dimension-independent syntax of an object selector.
///
/// This deliberately stops before catalog lookup.  A 2D or 3D lowerer may
/// assign different spatial meaning to tag expressions, but labels and marks
/// are part of the authoring language and must be parsed identically.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorSyntax {
    pub selector: String,
    pub base: String,
    pub tags: Vec<String>,
    pub occurrence_label: Option<String>,
    pub marks: Vec<SelectorMark>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectorSyntaxError {
    MarkMustEndWithBrace,
    MarkMissingSelector,
    NoMissingMark,
    InvalidMarkName,
    EmptyMarkValue,
    InvalidOccurrenceLabel,
}

impl SelectorSyntaxError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::MarkMustEndWithBrace => "mark selector must end with }",
            Self::MarkMissingSelector => "mark selector must attach to an object",
            Self::NoMissingMark => "`no` must be followed by a mark",
            Self::InvalidMarkName => {
                "mark name must start with an identifier and may use :value parts"
            }
            Self::EmptyMarkValue => "mark value must not be empty",
            Self::InvalidOccurrenceLabel => {
                "selector occurrence label must be: selector#label using only letters, numbers, and _"
            }
        }
    }
}

pub fn parse_selector_syntax(token: &str) -> Result<SelectorSyntax, SelectorSyntaxError> {
    let (selector, marks) = parse_selector_marks_syntax(token)?;
    let (selector, occurrence_label) = parse_selector_occurrence_label_syntax(selector)?;
    let mut parts = selector.split(':');
    let base = parts.next().unwrap_or_default().to_string();
    let tags = parts.map(str::to_string).collect();
    Ok(SelectorSyntax {
        selector,
        base,
        tags,
        occurrence_label,
        marks,
    })
}

fn parse_selector_marks_syntax(
    selector: &str,
) -> Result<(&str, Vec<SelectorMark>), SelectorSyntaxError> {
    let Some(open_index) = selector.find('{') else {
        return Ok((selector, Vec::new()));
    };
    let base = &selector[..open_index];
    let attrs = selector[open_index + 1..]
        .strip_suffix('}')
        .ok_or(SelectorSyntaxError::MarkMustEndWithBrace)?;
    if base.is_empty() {
        return Err(SelectorSyntaxError::MarkMissingSelector);
    }

    let mut marks = Vec::new();
    let mut tokens = attrs.split_whitespace().peekable();
    while let Some(token) = tokens.next() {
        let (negated, spec) = if token == "no" {
            (
                true,
                tokens.next().ok_or(SelectorSyntaxError::NoMissingMark)?,
            )
        } else {
            (false, token)
        };
        if mark_sugar_kind(spec).is_some() {
            marks.push(SelectorMark {
                name: String::new(),
                value: Some(spec.to_string()),
                negated,
            });
            continue;
        }
        let (name, value) = spec
            .split_once('=')
            .map_or((spec, None), |(name, value)| (name, Some(value)));
        let mut name_parts = name.split(':');
        let valid_name = name_parts.next().is_some_and(is_identifier)
            && name_parts.all(|part| {
                !part.is_empty()
                    && (matches!(part, ">" | "<" | "^" | "v")
                        || part
                            .chars()
                            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric()))
            });
        if !valid_name {
            return Err(SelectorSyntaxError::InvalidMarkName);
        }
        if value.is_some_and(str::is_empty) {
            return Err(SelectorSyntaxError::EmptyMarkValue);
        }
        marks.push(SelectorMark {
            name: name.to_string(),
            value: value.map(str::to_string),
            negated,
        });
    }
    Ok((base, marks))
}

fn parse_selector_occurrence_label_syntax(
    selector: &str,
) -> Result<(String, Option<String>), SelectorSyntaxError> {
    let (head, suffix) = selector.find(':').map_or((selector, ""), |colon| {
        (&selector[..colon], &selector[colon..])
    });
    let Some((base, label)) = head.split_once('#') else {
        return Ok((selector.to_string(), None));
    };
    if base.is_empty()
        || label.is_empty()
        || label.contains('#')
        || !label
            .chars()
            .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        return Err(SelectorSyntaxError::InvalidOccurrenceLabel);
    }
    Ok((format!("{base}{suffix}"), Some(label.to_string())))
}

/// Dimension-independent semantic occurrence used to lower a rewrite.
///
/// Spatial frontends own `Position`; selector resolution owns `Subject` and
/// `Mark`.  Presence, movement, and mark deltas are shared authoring semantics
/// and must not be reimplemented by a 2D or 3D frontend.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteOccurrence<Key, Position, Subject, Mark> {
    pub key: Key,
    pub position: Position,
    pub subject: Subject,
    pub require_marks: Vec<Mark>,
    pub forbid_marks: Vec<Mark>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RewriteOccurrenceDelta<Position, Subject, Mark> {
    Add {
        at: Position,
        subject: Subject,
    },
    Remove {
        at: Position,
        subject: Subject,
    },
    Move {
        from: Position,
        to: Position,
        subject: Subject,
    },
    SetMark {
        at: Position,
        subject: Subject,
        mark: Mark,
    },
    RemoveMark {
        at: Position,
        subject: Subject,
        mark: Mark,
    },
}

/// Computes the complete semantic delta between the two sides of a rewrite.
///
/// Direction expansion and rotation happen outside this function. Every
/// dimension must use this function after assigning stable occurrence keys.
pub fn diff_rewrite_occurrences<Key, Position, Subject, Mark, MarksEqual>(
    before: &[RewriteOccurrence<Key, Position, Subject, Mark>],
    after: &[RewriteOccurrence<Key, Position, Subject, Mark>],
    marks_equal: MarksEqual,
) -> Vec<RewriteOccurrenceDelta<Position, Subject, Mark>>
where
    Key: Eq,
    Position: Clone + Eq,
    Subject: Clone + Eq,
    Mark: Clone,
    MarksEqual: Fn(&Mark, &Mark) -> bool,
{
    let mut deltas = Vec::new();

    for before_occurrence in before {
        let Some(after_occurrence) = after
            .iter()
            .find(|after_occurrence| after_occurrence.key == before_occurrence.key)
        else {
            deltas.push(RewriteOccurrenceDelta::Remove {
                at: before_occurrence.position.clone(),
                subject: before_occurrence.subject.clone(),
            });
            continue;
        };

        if before_occurrence.subject != after_occurrence.subject {
            deltas.push(RewriteOccurrenceDelta::Remove {
                at: before_occurrence.position.clone(),
                subject: before_occurrence.subject.clone(),
            });
            deltas.push(RewriteOccurrenceDelta::Add {
                at: after_occurrence.position.clone(),
                subject: after_occurrence.subject.clone(),
            });
            continue;
        }

        if before_occurrence.position != after_occurrence.position {
            deltas.push(RewriteOccurrenceDelta::Move {
                from: before_occurrence.position.clone(),
                to: after_occurrence.position.clone(),
                subject: before_occurrence.subject.clone(),
            });
        }

        for mark in &after_occurrence.require_marks {
            if !before_occurrence
                .require_marks
                .iter()
                .any(|before_mark| marks_equal(before_mark, mark))
            {
                deltas.push(RewriteOccurrenceDelta::SetMark {
                    at: after_occurrence.position.clone(),
                    subject: after_occurrence.subject.clone(),
                    mark: mark.clone(),
                });
            }
        }
        for mark in &before_occurrence.require_marks {
            if !after_occurrence
                .require_marks
                .iter()
                .any(|after_mark| marks_equal(mark, after_mark))
            {
                deltas.push(RewriteOccurrenceDelta::RemoveMark {
                    at: after_occurrence.position.clone(),
                    subject: after_occurrence.subject.clone(),
                    mark: mark.clone(),
                });
            }
        }
        for mark in &after_occurrence.forbid_marks {
            deltas.push(RewriteOccurrenceDelta::RemoveMark {
                at: after_occurrence.position.clone(),
                subject: after_occurrence.subject.clone(),
                mark: mark.clone(),
            });
        }
    }

    for after_occurrence in after {
        if before
            .iter()
            .any(|before_occurrence| before_occurrence.key == after_occurrence.key)
        {
            continue;
        }
        deltas.push(RewriteOccurrenceDelta::Add {
            at: after_occurrence.position.clone(),
            subject: after_occurrence.subject.clone(),
        });
        for mark in &after_occurrence.require_marks {
            deltas.push(RewriteOccurrenceDelta::SetMark {
                at: after_occurrence.position.clone(),
                subject: after_occurrence.subject.clone(),
                mark: mark.clone(),
            });
        }
    }

    deltas
}

/// Builds stable rewrite occurrences for the shared selector representation.
/// Dimension-specific lowering supplies positions only.
pub fn selector_rewrite_occurrences<'a, Position: Clone>(
    cells: impl IntoIterator<Item = (Position, &'a [ObjectSelector<SelectorMark>])>,
) -> Vec<RewriteOccurrence<(String, usize), Position, ObjectSelector<SelectorMark>, SelectorMark>> {
    let mut ordinals = Vec::<(String, usize)>::new();
    let mut occurrences = Vec::new();
    for (position, selectors) in cells {
        for selector in selectors {
            let token = selector.token();
            let ordinal = if let Some((_, ordinal)) =
                ordinals.iter_mut().find(|(existing, _)| existing == &token)
            {
                let current = *ordinal;
                *ordinal += 1;
                current
            } else {
                ordinals.push((token.clone(), 1));
                0
            };
            let marks = selector.mark();
            occurrences.push(RewriteOccurrence {
                key: (token, ordinal),
                position: position.clone(),
                subject: selector_without_marks(selector),
                require_marks: marks.iter().filter(|mark| !mark.negated).cloned().collect(),
                forbid_marks: marks
                    .iter()
                    .filter(|mark| mark.negated)
                    .map(|mark| SelectorMark {
                        name: mark.name.clone(),
                        value: mark.value.clone(),
                        negated: false,
                    })
                    .collect(),
            });
        }
    }
    occurrences
}

fn selector_without_marks(selector: &ObjectSelector<SelectorMark>) -> ObjectSelector<SelectorMark> {
    match selector {
        ObjectSelector::WithMark { selector, .. } => selector_without_marks(selector),
        ObjectSelector::Labeled { token, selector } => {
            ObjectSelector::labeled(token.clone(), selector_without_marks(selector))
        }
        _ => selector.clone(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConcreteObject<ObjectId> {
    pub id: ObjectId,
    pub name: String,
}

impl<ObjectId> ConcreteObject<ObjectId> {
    pub fn new(id: ObjectId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorGroup<Selector> {
    pub name: String,
    pub selectors: Vec<Selector>,
}

impl<Selector> SelectorGroup<Selector> {
    pub fn new(name: impl Into<String>, selectors: Vec<Selector>) -> Self {
        Self {
            name: name.into(),
            selectors,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectFamily<ObjectId, Axis> {
    pub name: String,
    pub axes: Vec<Axis>,
    pub variants: Vec<ObjectVariant<ObjectId>>,
}

impl<ObjectId, Axis> ObjectFamily<ObjectId, Axis> {
    pub fn new(
        name: impl Into<String>,
        axes: Vec<Axis>,
        variants: Vec<ObjectVariant<ObjectId>>,
    ) -> Self {
        Self {
            name: name.into(),
            axes,
            variants,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectVariant<ObjectId> {
    pub id: ObjectId,
    pub values: Vec<String>,
}

impl<ObjectId> ObjectVariant<ObjectId> {
    pub fn new(id: ObjectId, values: Vec<impl Into<String>>) -> Self {
        Self {
            id,
            values: values.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ObjectSelector<Mark> {
    Any,
    Object(String),
    Group(String),
    Labeled {
        token: String,
        selector: Box<ObjectSelector<Mark>>,
    },
    Variant {
        family: String,
        tags: Vec<SelectorTag>,
    },
    WithMark {
        selector: Box<ObjectSelector<Mark>>,
        mark: Vec<Mark>,
    },
}

impl<Mark> ObjectSelector<Mark> {
    pub const fn any() -> Self {
        Self::Any
    }

    pub fn object(name: impl Into<String>) -> Self {
        Self::Object(name.into())
    }

    pub fn group(name: impl Into<String>) -> Self {
        Self::Group(name.into())
    }

    pub fn labeled(token: impl Into<String>, selector: ObjectSelector<Mark>) -> Self {
        Self::Labeled {
            token: token.into(),
            selector: Box::new(selector),
        }
    }

    pub fn variant(family: impl Into<String>, tags: Vec<SelectorTag>) -> Self {
        Self::Variant {
            family: family.into(),
            tags,
        }
    }

    pub fn with_mark(selector: ObjectSelector<Mark>, mark: Vec<Mark>) -> Self {
        if mark.is_empty() {
            selector
        } else {
            Self::WithMark {
                selector: Box::new(selector),
                mark,
            }
        }
    }

    pub fn token(&self) -> String {
        match self {
            Self::Any => "*".to_string(),
            Self::Object(name) | Self::Group(name) => name.clone(),
            Self::Labeled { token, .. } => token.clone(),
            Self::WithMark { selector, .. } => selector.token(),
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
            || matches!(self, Self::WithMark { selector, .. } if selector.has_occurrence_label())
    }

    pub fn mark(&self) -> &[Mark] {
        match self {
            Self::WithMark { mark, .. } => mark,
            _ => &[],
        }
    }

    pub fn can_use_runtime_object_set(&self) -> bool {
        matches!(self, Self::Group(_))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectorTag {
    Value(String),
    Any,
}

impl SelectorTag {
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
pub enum SelectorCatalogError {
    DuplicateObjectName { name: String },
    DuplicateFamilyName { name: String },
    DuplicateGroupName { name: String },
    ObjectNameShadowsFamily { name: String },
    FamilyNameShadowsObject { name: String },
    GroupNameShadowsSelector { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectorError {
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

fn default_object_layers<ObjectId, LayerId, Axis>(
    objects: &[ConcreteObject<ObjectId>],
    families: &[ObjectFamily<ObjectId, Axis>],
) -> Vec<(ObjectId, LayerId)>
where
    ObjectId: Copy,
    LayerId: Default,
{
    objects
        .iter()
        .map(|object| object.id)
        .chain(
            families
                .iter()
                .flat_map(|family| family.variants.iter().map(|variant| variant.id)),
        )
        .map(|object| (object, LayerId::default()))
        .collect()
}

fn validate_catalog_names<ObjectId, Axis, Mark>(
    objects: &[ConcreteObject<ObjectId>],
    families: &[ObjectFamily<ObjectId, Axis>],
    groups: &[SelectorGroup<ObjectSelector<Mark>>],
) -> Result<(), SelectorCatalogError>
where
    Axis: VariantAxisSpec,
{
    let mut object_names = Vec::<String>::new();
    for object in objects {
        if object_names.contains(&object.name) {
            return Err(SelectorCatalogError::DuplicateObjectName {
                name: object.name.clone(),
            });
        }
        object_names.push(object.name.clone());
    }

    let mut family_names = Vec::<String>::new();
    for family in families {
        if family_names.contains(&family.name) {
            return Err(SelectorCatalogError::DuplicateFamilyName {
                name: family.name.clone(),
            });
        }
        if object_names.contains(&family.name) {
            return Err(SelectorCatalogError::FamilyNameShadowsObject {
                name: family.name.clone(),
            });
        }
        family_names.push(family.name.clone());
    }

    for object_name in &object_names {
        if family_names.contains(object_name) {
            return Err(SelectorCatalogError::ObjectNameShadowsFamily {
                name: object_name.clone(),
            });
        }
    }

    let mut group_names = Vec::<String>::new();
    for group in groups {
        if group_names.contains(&group.name) {
            return Err(SelectorCatalogError::DuplicateGroupName {
                name: group.name.clone(),
            });
        }
        if object_names.contains(&group.name)
            || family_names.contains(&group.name)
            || group_names.contains(&group.name)
        {
            return Err(SelectorCatalogError::GroupNameShadowsSelector {
                name: group.name.clone(),
            });
        }
        group_names.push(group.name.clone());
    }

    Ok(())
}

fn expanded_tags<ObjectId, Axis>(
    family: &ObjectFamily<ObjectId, Axis>,
    tags: &[SelectorTag],
) -> Result<Vec<SelectorTag>, SelectorError> {
    if tags.is_empty() {
        return Err(SelectorError::BareVariantFamily {
            family: family.name.clone(),
        });
    }
    if tags.len() > family.axes.len() {
        return Err(SelectorError::WrongVariantArity {
            family: family.name.clone(),
            expected: family.axes.len(),
            actual: tags.len(),
        });
    }
    if tags.len() == 1 && tags[0] == SelectorTag::Any {
        return Ok(vec![SelectorTag::Any; family.axes.len()]);
    }
    if tags.len() < family.axes.len() {
        return Err(SelectorError::PartialVariantSelector {
            family: family.name.clone(),
            expected: family.axes.len(),
            actual: tags.len(),
        });
    }
    Ok(tags.to_vec())
}

fn push_unique<T: Eq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

pub fn new_puzzle_source(_title: &str) -> String {
    String::new()
}

pub fn is_at_identifier_token(token: &str) -> bool {
    let Some(rest) = token.strip_prefix('@') else {
        return false;
    };
    let without_mark = rest.split_once('{').map_or(rest, |(base, _)| base);
    let base = without_mark
        .split_once(':')
        .map_or(without_mark, |(base, _)| base);
    is_identifier(base)
}

pub fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return false;
    }
    chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

pub fn is_qualified_identifier(value: &str) -> bool {
    let mut parts = value.split(':');
    let Some(first) = parts.next() else {
        return false;
    };
    is_identifier(first) && parts.all(is_identifier)
}

pub fn split_object_spec(token: &str) -> Option<(&str, impl Iterator<Item = &str> + '_)> {
    let mut parts = token.split(':');
    let base = parts.next()?;
    (!base.is_empty()).then_some((base, parts))
}

pub fn split_header_tokens(line: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = None::<usize>;
    let mut paren_depth = 0_u16;
    for (index, ch) in line.char_indices() {
        match ch {
            '(' => {
                start.get_or_insert(index);
                paren_depth += 1;
            }
            ')' => {
                start.get_or_insert(index);
                paren_depth = paren_depth.saturating_sub(1);
            }
            ch if ch.is_whitespace() && paren_depth == 0 => {
                if let Some(token_start) = start.take() {
                    tokens.push(&line[token_start..index]);
                }
            }
            _ => {
                start.get_or_insert(index);
            }
        }
    }
    if let Some(token_start) = start {
        tokens.push(&line[token_start..]);
    }
    if tokens.len() > 1 && tokens.last().copied() == Some("{") {
        tokens.pop();
    }
    tokens
}

pub fn parse_quoted_text(value: &str) -> Option<String> {
    let inner = value.strip_prefix('"')?.strip_suffix('"')?;
    Some(inner.replace("\\\"", "\""))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallSurface<'a> {
    pub name: &'a str,
    pub args: Vec<&'a str>,
}

pub fn parse_assignment_row(line: &str) -> Option<(&str, &str)> {
    for (index, ch) in top_level_scan(line) {
        let previous = line[..index].chars().next_back();
        let next = line[index + ch.len_utf8()..].chars().next();
        if ch == '='
            && !matches!(previous, Some('=' | '!' | '<' | '>'))
            && !matches!(next, Some('='))
        {
            return Some((line[..index].trim(), line[index + ch.len_utf8()..].trim()));
        }
    }
    None
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorAssignmentSurface<'a> {
    pub name: &'a str,
    pub selectors: Vec<&'a str>,
}

pub fn selector_assignment_surface(line: &str) -> Option<SelectorAssignmentSurface<'_>> {
    let (name, selectors) = parse_assignment_row(line)?;
    let name_tokens = split_header_tokens(name);
    let selector_tokens = split_header_tokens(selectors);
    let [name] = name_tokens.as_slice() else {
        return None;
    };
    if selector_tokens.is_empty() {
        return None;
    }
    Some(SelectorAssignmentSurface {
        name,
        selectors: selector_tokens,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotRowSurface<'a> {
    Anonymous { selectors: Vec<&'a str> },
    Named(SelectorAssignmentSurface<'a>),
    Each { selectors: Vec<&'a str> },
}

pub fn slot_row_surface(line: &str) -> Option<SlotRowSurface<'_>> {
    if let Some(assignment) = selector_assignment_surface(line) {
        return Some(SlotRowSurface::Named(assignment));
    }
    let tokens = split_header_tokens(line);
    match tokens.as_slice() {
        [] | ["each"] => None,
        ["each", selectors @ ..] => Some(SlotRowSurface::Each {
            selectors: selectors.to_vec(),
        }),
        selectors => Some(SlotRowSurface::Anonymous {
            selectors: selectors.to_vec(),
        }),
    }
}

pub fn selector_alias_conflicts<'a>(
    name: &str,
    object_names: impl IntoIterator<Item = &'a str>,
    family_names: impl IntoIterator<Item = &'a str>,
    group_names: impl IntoIterator<Item = &'a str>,
) -> bool {
    object_names
        .into_iter()
        .chain(family_names)
        .chain(group_names)
        .any(|candidate| candidate == name)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PuzzleDirectiveSurface {
    Empty,
    Close,
    Model,
    RemovedModelPrefix,
    Metadata,
    DocumentShell,
    Slots,
    Marks,
    Tags,
    Map,
    Objects,
    Keys,
    Inputs,
    Groups,
    SingularGroup,
    Legend,
    Levels,
    Level,
    RemovedLevels3,
    Sprites,
    Scene,
    RuleProgram,
    WinConditions,
    Query,
    Solver,
    Render,
    Assignment,
    Unknown,
}

impl PuzzleDirectiveSurface {
    pub const fn is_catalog_owned(self) -> bool {
        matches!(
            self,
            Self::Slots | Self::Marks | Self::Tags | Self::Map | Self::Groups
        )
    }
}

pub fn puzzle_directive_surface(line: &str) -> PuzzleDirectiveSurface {
    let line = line.trim();
    if line.is_empty() {
        return PuzzleDirectiveSurface::Empty;
    }
    if line == "}" {
        return PuzzleDirectiveSurface::Close;
    }
    if rule_program_block_surface(line).is_some() {
        return PuzzleDirectiveSurface::RuleProgram;
    }
    let tokens = split_header_tokens(line);
    let Some(first) = tokens.first().copied() else {
        return PuzzleDirectiveSurface::Unknown;
    };
    match first {
        "puzzle" => PuzzleDirectiveSurface::Model,
        "model" => PuzzleDirectiveSurface::RemovedModelPrefix,
        "title" | "subtitle" | "author" | "homepage" | "default_wait_time" | "again_interval" => {
            PuzzleDirectiveSurface::Metadata
        }
        "theme" if parse_assignment_row(line).is_some() => PuzzleDirectiveSurface::Metadata,
        "sounds" | "theme" | "assets" => PuzzleDirectiveSurface::DocumentShell,
        "slots" => PuzzleDirectiveSurface::Slots,
        "marks" => PuzzleDirectiveSurface::Marks,
        "tags" => PuzzleDirectiveSurface::Tags,
        "map" => PuzzleDirectiveSurface::Map,
        "objects" => PuzzleDirectiveSurface::Objects,
        "keys" => PuzzleDirectiveSurface::Keys,
        "inputs" => PuzzleDirectiveSurface::Inputs,
        "groups" => PuzzleDirectiveSurface::Groups,
        "group" => PuzzleDirectiveSurface::SingularGroup,
        "legend" => PuzzleDirectiveSurface::Legend,
        "levels" => PuzzleDirectiveSurface::Levels,
        "level" => PuzzleDirectiveSurface::Level,
        "levels3" => PuzzleDirectiveSurface::RemovedLevels3,
        "sprites" => PuzzleDirectiveSurface::Sprites,
        "scene" => PuzzleDirectiveSurface::Scene,
        "win_conditions" => PuzzleDirectiveSurface::WinConditions,
        "query" => PuzzleDirectiveSurface::Query,
        "solver" => PuzzleDirectiveSurface::Solver,
        "render" => PuzzleDirectiveSurface::Render,
        _ if parse_assignment_row(line).is_some() => PuzzleDirectiveSurface::Assignment,
        _ => PuzzleDirectiveSurface::Unknown,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceHeaderSurface<'a> {
    pub name: Option<&'a str>,
    pub owner: Option<&'a str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceBlockSurface<'a> {
    pub header: ResourceHeaderSurface<'a>,
    pub body_start: usize,
    pub body_end: usize,
    pub next_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceHeaderSurfaceError {
    message: String,
}

impl ResourceHeaderSurfaceError {
    pub fn message(&self) -> &str {
        &self.message
    }
}

pub fn resource_header_surface<'a>(
    line: &'a str,
    keyword: &str,
) -> Result<ResourceHeaderSurface<'a>, ResourceHeaderSurfaceError> {
    let tokens = split_header_tokens(line);
    let (name, owner) = match tokens.as_slice() {
        [head] if *head == keyword => (None, None),
        [head, "of", owner] if *head == keyword => (None, Some(*owner)),
        [head, name] if *head == keyword => (Some(*name), None),
        [head, name, "of", owner] if *head == keyword => (Some(*name), Some(*owner)),
        _ => {
            return Err(resource_header_error(format!(
                "{keyword} header must be: {keyword} [name] [of owner] {{"
            )));
        }
    };
    if name.is_some_and(|name| !is_qualified_identifier(name)) {
        return Err(resource_header_error(format!(
            "{keyword} resource name must be a qualified identifier"
        )));
    }
    if owner.is_some_and(|owner| !is_qualified_identifier(owner)) {
        return Err(resource_header_error(format!(
            "{keyword} owner must be a qualified identifier"
        )));
    }
    Ok(ResourceHeaderSurface { name, owner })
}

fn resource_header_error(message: String) -> ResourceHeaderSurfaceError {
    ResourceHeaderSurfaceError { message }
}

pub fn collect_resource_block_surface<'a>(
    lines: &'a [String],
    header_index: usize,
    keyword: &str,
) -> Result<ResourceBlockSurface<'a>, ResourceHeaderSurfaceError> {
    let header_line = lines
        .get(header_index)
        .ok_or_else(|| resource_header_error(format!("{keyword} resource header is missing")))?;
    if !header_line.trim_end().ends_with('{') {
        return Err(resource_header_error(format!(
            "{keyword} block must end with {{"
        )));
    }
    let header = resource_header_surface(header_line, keyword)?;
    let body_start = header_index + 1;
    let block = collect_container_block_surface(lines, body_start, keyword)
        .map_err(|error| resource_header_error(error.message().to_string()))?;
    Ok(ResourceBlockSurface {
        header,
        body_start: block.body_start,
        body_end: block.body_end,
        next_index: block.next_index,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WinConditionQuantifier {
    Exists,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WinConditionRowSurface<'a> {
    Query {
        quantifier: WinConditionQuantifier,
        argument: &'a str,
    },
    SomeOn {
        subject: &'a str,
        cover: &'a str,
    },
    AllOn {
        subject: &'a str,
        cover: &'a str,
    },
    Expression(&'a str),
}

pub fn win_condition_row_surface(line: &str) -> Result<WinConditionRowSurface<'_>, &'static str> {
    let line = line.trim();
    if let Some((call, suffix)) = parse_optional_call_surface_with_suffix(line)
        .map_err(|_| "win condition call has unbalanced parentheses")?
    {
        if !suffix.is_empty() {
            return Ok(WinConditionRowSurface::Expression(line));
        }
        let quantifier = match call.name {
            "exists" | "some" => WinConditionQuantifier::Exists,
            "none" | "no" => WinConditionQuantifier::None,
            _ => return Ok(WinConditionRowSurface::Expression(line)),
        };
        let [argument] = call.args.as_slice() else {
            return Err("win condition query must have exactly one argument");
        };
        return Ok(WinConditionRowSurface::Query {
            quantifier,
            argument: argument.trim(),
        });
    }

    let tokens = split_header_tokens(line);
    Ok(match tokens.as_slice() {
        ["all", subject, "on", cover] => WinConditionRowSurface::AllOn { subject, cover },
        ["some", subject, "on", cover] => WinConditionRowSurface::SomeOn { subject, cover },
        ["some", argument @ ..] if !argument.is_empty() => WinConditionRowSurface::Query {
            quantifier: WinConditionQuantifier::Exists,
            argument: line.strip_prefix("some").unwrap_or_default().trim(),
        },
        ["no", argument @ ..] if !argument.is_empty() => WinConditionRowSurface::Query {
            quantifier: WinConditionQuantifier::None,
            argument: line.strip_prefix("no").unwrap_or_default().trim(),
        },
        _ => WinConditionRowSurface::Expression(line),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowBlockSurface<'a> {
    pub rows: Vec<&'a str>,
    pub next_index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowBlockSurfaceError {
    message: String,
}

impl RowBlockSurfaceError {
    pub fn message(&self) -> &str {
        &self.message
    }
}

pub fn collect_row_block_surface<'a>(
    lines: &'a [String],
    body_start: usize,
    owner: &str,
) -> Result<RowBlockSurface<'a>, RowBlockSurfaceError> {
    let mut rows = Vec::new();
    let mut index = body_start;
    while let Some(line) = lines.get(index) {
        if line == "}" {
            return Ok(RowBlockSurface {
                rows,
                next_index: index + 1,
            });
        }
        if line.ends_with('{') {
            return Err(RowBlockSurfaceError {
                message: format!("{owner} accepts rows, not nested blocks: {line}"),
            });
        }
        if !line.is_empty() {
            rows.push(line.as_str());
        }
        index += 1;
    }
    Err(RowBlockSurfaceError {
        message: format!("{owner} block missing }}"),
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContainerBlockSurface {
    pub body_start: usize,
    pub body_end: usize,
    pub next_index: usize,
}

pub fn collect_container_block_surface(
    lines: &[String],
    body_start: usize,
    owner: &str,
) -> Result<ContainerBlockSurface, RowBlockSurfaceError> {
    let mut depth = 1usize;
    let mut index = body_start;
    while let Some(line) = lines.get(index) {
        if line == "}" {
            depth -= 1;
            if depth == 0 {
                return Ok(ContainerBlockSurface {
                    body_start,
                    body_end: index,
                    next_index: index + 1,
                });
            }
        } else if line.ends_with('{') {
            depth += 1;
        }
        index += 1;
    }
    Err(RowBlockSurfaceError {
        message: format!("{owner} block missing }}"),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorGroupDeclaration {
    pub name: String,
    pub selectors: Vec<String>,
    pub source_line: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExpandedLayerSelectors {
    pub terms: Vec<String>,
    pub used_groups: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayerSelectorExpansionError {
    pub source_line: String,
    pub message: &'static str,
}

pub fn expand_layer_selectors(
    selectors: &[impl AsRef<str>],
    groups: &[SelectorGroupDeclaration],
) -> Result<ExpandedLayerSelectors, LayerSelectorExpansionError> {
    let mut expanded = ExpandedLayerSelectors {
        terms: Vec::new(),
        used_groups: Vec::new(),
    };
    let mut resolving = Vec::<String>::new();
    for selector in selectors {
        expand_layer_selector(selector.as_ref(), groups, &mut resolving, &mut expanded)?;
    }
    Ok(expanded)
}

fn expand_layer_selector(
    selector: &str,
    groups: &[SelectorGroupDeclaration],
    resolving: &mut Vec<String>,
    expanded: &mut ExpandedLayerSelectors,
) -> Result<(), LayerSelectorExpansionError> {
    let Some(group) = groups.iter().find(|group| group.name == selector) else {
        expanded.terms.push(selector.to_string());
        return Ok(());
    };
    if resolving.iter().any(|candidate| candidate == selector) {
        return Err(LayerSelectorExpansionError {
            source_line: group.source_line.clone(),
            message: "group definitions cannot be cyclic",
        });
    }
    if !expanded.used_groups.contains(&group.name) {
        expanded.used_groups.push(group.name.clone());
    }
    resolving.push(group.name.clone());
    for selector in &group.selectors {
        expand_layer_selector(selector, groups, resolving, expanded)?;
    }
    resolving.pop();
    Ok(())
}

pub fn parse_optional_call_surface_with_suffix<'a>(
    value: &'a str,
) -> Result<Option<(CallSurface<'a>, &'a str)>, ()> {
    let value = value.trim();
    let Some(open) = find_top_level_char(value, '(') else {
        return Ok(None);
    };
    let close = matching_delimiter(value, open, '(', ')').ok_or(())?;
    let name = value[..open].trim();
    let args = parse_call_argument_surfaces(&value[open + 1..close]);
    let suffix = value[close + 1..].trim();
    Ok(Some((CallSurface { name, args }, suffix)))
}

pub fn parse_call_argument_surfaces(value: &str) -> Vec<&str> {
    if value.trim().is_empty() {
        return Vec::new();
    }
    split_top_level_commas(value)
        .into_iter()
        .map(str::trim)
        .collect()
}

pub fn parse_view_path(value: &str) -> Option<Vec<String>> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.is_empty() || !parts.iter().all(|part| is_qualified_identifier(part)) {
        return None;
    }
    Some(parts.into_iter().map(ToString::to_string).collect())
}

pub fn split_top_level_keyword_once<'a>(
    value: &'a str,
    keyword: &str,
) -> Option<(&'a str, &'a str)> {
    for (index, _) in top_level_scan(value) {
        if !value[index..].starts_with(keyword) {
            continue;
        }
        let before = value[..index].chars().next_back();
        let after = value[index + keyword.len()..].chars().next();
        if before.is_none_or(|ch| !is_identifier_continue(ch))
            && after.is_none_or(|ch| !is_identifier_continue(ch))
        {
            return Some((&value[..index], &value[index + keyword.len()..]));
        }
    }
    None
}

pub fn split_top_level_operator_once<'a>(
    value: &'a str,
    operator: &str,
) -> Option<(&'a str, &'a str)> {
    for (index, _) in top_level_scan(value) {
        if value[index..].starts_with(operator) {
            return Some((&value[..index], &value[index + operator.len()..]));
        }
    }
    None
}

pub fn find_top_level_char(value: &str, target: char) -> Option<usize> {
    top_level_char_indexes(value, target).into_iter().next()
}

pub fn matching_delimiter(
    value: &str,
    open: usize,
    open_ch: char,
    close_ch: char,
) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in value[open..].char_indices() {
        let index = open + index;
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            _ if ch == open_ch => depth += 1,
            _ if ch == close_ch => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level_commas(value: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    for index in top_level_char_indexes(value, ',') {
        parts.push(&value[start..index]);
        start = index + 1;
    }
    parts.push(&value[start..]);
    parts
}

fn top_level_char_indexes(value: &str, target: char) -> Vec<usize> {
    top_level_scan(value)
        .into_iter()
        .filter_map(|(index, ch)| (ch == target).then_some(index))
        .collect()
}

fn top_level_scan(value: &str) -> Vec<(usize, char)> {
    let mut out = Vec::new();
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in value.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '(' => {
                if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                    out.push((index, ch));
                }
                paren_depth += 1;
            }
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '{' => {
                if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                    out.push((index, ch));
                }
                brace_depth += 1;
            }
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => {
                if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 {
                    out.push((index, ch));
                }
                bracket_depth += 1;
            }
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            _ if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                out.push((index, ch));
            }
            _ => {}
        }
    }
    out
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelHeaderNameError {
    InvalidSyntax,
    EmptyName,
}

impl LevelHeaderNameError {
    pub fn message(self) -> &'static str {
        match self {
            Self::InvalidSyntax => "level header must be: level \"<id>\"",
            Self::EmptyName => "level id must not be empty",
        }
    }
}

pub fn parse_level_header_name_or_auto(
    line: &str,
    auto_name: String,
) -> Result<String, LevelHeaderNameError> {
    let Some(rest) = line.trim().strip_prefix("level") else {
        return Err(LevelHeaderNameError::InvalidSyntax);
    };
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return Err(LevelHeaderNameError::InvalidSyntax);
    }
    let name_text = strip_level_header_block_opener(rest.trim()).trim();
    if name_text.is_empty() {
        return Ok(auto_name);
    }
    let Some(name) = parse_quoted_text(name_text) else {
        return Err(LevelHeaderNameError::InvalidSyntax);
    };
    if name.is_empty() {
        return Err(LevelHeaderNameError::EmptyName);
    }
    Ok(name)
}

pub fn strip_level_header_block_opener(value: &str) -> &str {
    value.strip_suffix('{').map(str::trim_end).unwrap_or(value)
}

pub fn is_braced_level_header(line: &str) -> bool {
    line.trim_end().ends_with('{') && matches!(split_header_tokens(line).as_slice(), ["level", ..])
}

pub fn unnamed_level_name(existing_count: usize) -> String {
    format!("unnamed level {}", existing_count + 1)
}

pub fn namespaced_unnamed_level_name(
    namespace: Option<&str>,
    existing_count: usize,
    namespace_count: usize,
) -> String {
    match namespace {
        Some(namespace) => format!("{namespace}.{namespace_count}"),
        None => unnamed_level_name(existing_count),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyBindingSurface<'a> {
    pub keys: Vec<&'a str>,
    pub target: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyBindingSurfaceError {
    UseArrow,
    MissingTarget,
    MissingKeys,
}

impl KeyBindingSurfaceError {
    pub fn message(self) -> &'static str {
        match self {
            Self::UseArrow => "keys row must use `->`: <key...> -> <input-or-command>",
            Self::MissingTarget => "keys row must name a target after ->",
            Self::MissingKeys => "keys row must include at least one key before ->",
        }
    }
}

pub fn key_binding_surface(line: &str) -> Result<KeyBindingSurface<'_>, KeyBindingSurfaceError> {
    if line.contains('=') {
        return Err(KeyBindingSurfaceError::UseArrow);
    }
    let (keys, target) = line
        .split_once("->")
        .ok_or(KeyBindingSurfaceError::UseArrow)?;
    let target = target.trim();
    if target.is_empty() {
        return Err(KeyBindingSurfaceError::MissingTarget);
    }
    let keys = keys.split_whitespace().collect::<Vec<_>>();
    if keys.is_empty() {
        return Err(KeyBindingSurfaceError::MissingKeys);
    }
    Ok(KeyBindingSurface { keys, target })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleProgramBlockSurface<'a> {
    Rules { modifier: &'a str },
    OnLevelStart { modifier: &'a str },
    OnLevelClear,
    OnLastLevelClear,
}

pub fn rule_program_block_surface(line: &str) -> Option<RuleProgramBlockSurface<'_>> {
    if let Some(modifier) = named_block_header_modifier(line, "rules") {
        return Some(RuleProgramBlockSurface::Rules { modifier });
    }
    if let Some(modifier) = named_block_header_modifier(line, "on_level_start") {
        return Some(RuleProgramBlockSurface::OnLevelStart { modifier });
    }
    if named_block_header_modifier(line, "on_level_clear").is_some() {
        return Some(RuleProgramBlockSurface::OnLevelClear);
    }
    if named_block_header_modifier(line, "on_last_level_clear").is_some() {
        return Some(RuleProgramBlockSurface::OnLastLevelClear);
    }
    None
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleProgramBlockBody {
    RuleStatements(Vec<RuleStatementSyntax>),
    LifecycleCommands(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleStatementSyntax {
    Line(String),
    Block {
        header: String,
        statements: Vec<RuleStatementSyntax>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleProgramBlockBodyError {
    MissingClosingBrace { block_name: &'static str },
}

impl RuleProgramBlockBodyError {
    pub fn message(self) -> String {
        match self {
            Self::MissingClosingBrace { block_name } => {
                format!("{block_name} block missing }}")
            }
        }
    }
}

/// Collects a rule-program body whose owner boundary has already been parsed.
///
/// `lines` contains only the direct body of the owning block, without its
/// closing brace. Nested statement blocks still carry their own braces.
pub fn collect_rule_program_entry_body(
    lines: &[String],
    block: RuleProgramBlockSurface<'_>,
) -> Result<RuleProgramBlockBody, RuleProgramBlockBodyError> {
    match block {
        RuleProgramBlockSurface::Rules { .. } => collect_rule_statement_entry_body(lines, "rules")
            .map(RuleProgramBlockBody::RuleStatements),
        RuleProgramBlockSurface::OnLevelStart { .. } => {
            collect_rule_statement_entry_body(lines, "on_level_start")
                .map(RuleProgramBlockBody::RuleStatements)
        }
        RuleProgramBlockSurface::OnLevelClear => Ok(RuleProgramBlockBody::LifecycleCommands(
            collect_lifecycle_command_entry_body(lines),
        )),
        RuleProgramBlockSurface::OnLastLevelClear => Ok(RuleProgramBlockBody::LifecycleCommands(
            collect_lifecycle_command_entry_body(lines),
        )),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleStatementBlockSurface<'a> {
    Program(RuleProgramBlockSurface<'a>),
    Routine,
    Nested,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleRoutineBlockHeaderSurfaceSpans {
    pub keyword: Range<usize>,
    pub name: Option<Range<usize>>,
    pub modifiers: Vec<Range<usize>>,
}

pub fn rule_routine_block_header_surface_spans(
    line: &str,
) -> Option<RuleRoutineBlockHeaderSurfaceSpans> {
    let tokens = header_token_spans(line, trimmed_range(line));
    let keyword = tokens.first()?;
    if keyword.text != "routine" {
        return None;
    }
    let name = tokens.get(1).map(|token| token.range.clone());
    let modifiers = tokens
        .iter()
        .skip(2)
        .filter(|token| rule_application_surface(token.text).is_some())
        .map(|token| token.range.clone())
        .collect::<Vec<_>>();
    Some(RuleRoutineBlockHeaderSurfaceSpans {
        keyword: keyword.range.clone(),
        name,
        modifiers,
    })
}

pub fn rule_statement_block_surface(
    line: &str,
    parent_is_statement_block: bool,
) -> Option<RuleStatementBlockSurface<'_>> {
    let trimmed = line.trim();
    trimmed.strip_suffix('{')?;
    if let Some(program) = rule_program_block_surface(trimmed) {
        return Some(RuleStatementBlockSurface::Program(program));
    }
    let tokens = split_header_tokens(trimmed);
    match tokens.first().copied()? {
        "routine" => Some(RuleStatementBlockSurface::Routine),
        _ if parent_is_statement_block && nested_rule_statement_block_surface(trimmed, &tokens) => {
            Some(RuleStatementBlockSurface::Nested)
        }
        _ => None,
    }
}

fn nested_rule_statement_block_surface(line: &str, tokens: &[&str]) -> bool {
    if line
        .strip_suffix('{')
        .map(str::trim_end)
        .is_some_and(|head| head.contains("->"))
    {
        return true;
    }
    match tokens.first().copied() {
        Some("display" | "else" | "fix" | "for" | "if") => true,
        Some("repeat") if tokens.get(1).copied() == Some("until") => true,
        Some(first) => rule_application_surface(first).is_some(),
        None => false,
    }
}

fn named_block_header_modifier<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let head = line.trim().strip_suffix('{')?.trim_end();
    let rest = head.strip_prefix(keyword)?;
    if rest.is_empty() {
        return Some("");
    }
    rest.strip_prefix(' ').map(str::trim)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleApplicationSurface {
    Once,
    OnceAll,
    OncePerLevel,
    Random,
    Repeat,
}

pub const RULE_STATEMENT_HEAD_KEYWORDS: &[&str] = &[
    "display",
    "for",
    "if",
    "input",
    "once",
    "once_all",
    "once_per_level",
    "random",
    "repeat",
];

pub fn rule_application_surface(token: &str) -> Option<RuleApplicationSurface> {
    match token {
        "once" => Some(RuleApplicationSurface::Once),
        "once_all" => Some(RuleApplicationSurface::OnceAll),
        "once_per_level" => Some(RuleApplicationSurface::OncePerLevel),
        "random" => Some(RuleApplicationSurface::Random),
        "repeat" => Some(RuleApplicationSurface::Repeat),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleStatementSurface<'a> {
    ApplicationBlock { application: RuleApplicationSurface },
    RuleLine(RuleLineSurface<'a>),
    Call { name: &'a str },
}

pub fn rule_statement_surface(
    line: &str,
) -> Result<RuleStatementSurface<'_>, RuleLineSurfaceError> {
    let line = line.trim();
    let tokens = split_header_tokens(line);
    if let [application] = tokens.as_slice()
        && let Some(application) = rule_application_surface(application)
    {
        return Ok(RuleStatementSurface::ApplicationBlock { application });
    }
    if tokens.len() == 1 && is_qualified_identifier(tokens[0]) {
        return Ok(RuleStatementSurface::Call { name: tokens[0] });
    }
    rule_line_surface(line).map(RuleStatementSurface::RuleLine)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleLineSurface<'a> {
    InputRewrite {
        application: Option<RuleApplicationSurface>,
        surface: InputRewriteSurface<'a>,
    },
    NeutralRewrite {
        application: Option<RuleApplicationSurface>,
        rewrite: &'a str,
    },
    OrientedRewrite {
        application: Option<RuleApplicationSurface>,
        orientation: &'a str,
        rewrite: &'a str,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleApplicationSurfaceSpan {
    pub application: RuleApplicationSurface,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputRewriteSurfaceSpans {
    pub input: Range<usize>,
    pub orientation: Option<Range<usize>>,
    pub rewrite: Range<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuleLineSurfaceSpans {
    InputRewrite {
        application: Option<RuleApplicationSurfaceSpan>,
        surface: InputRewriteSurfaceSpans,
    },
    NeutralRewrite {
        application: Option<RuleApplicationSurfaceSpan>,
        rewrite: Range<usize>,
    },
    OrientedRewrite {
        application: Option<RuleApplicationSurfaceSpan>,
        orientation: Range<usize>,
        rewrite: Range<usize>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleSemanticSurfaceKind {
    Direction,
    Keyword,
    Object,
    Mark,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleSemanticSurfaceSpan {
    pub kind: RuleSemanticSurfaceKind,
    pub span: Range<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleLineSurfaceError {
    Input(InputRewriteSurfaceError),
    MissingOrientation,
    RewriteMustStartWithPattern,
}

impl RuleLineSurfaceError {
    pub fn message(self) -> &'static str {
        match self {
            Self::Input(error) => error.message(),
            Self::MissingOrientation | Self::RewriteMustStartWithPattern => {
                "rule must be: <orientation> [ ... ] -> [ ... ]"
            }
        }
    }
}

pub fn rule_line_surface(line: &str) -> Result<RuleLineSurface<'_>, RuleLineSurfaceError> {
    Ok(match rule_line_surface_spans(line)? {
        RuleLineSurfaceSpans::InputRewrite {
            application,
            surface,
        } => RuleLineSurface::InputRewrite {
            application: application.map(|application| application.application),
            surface: InputRewriteSurface {
                orientation: surface.orientation.map(|range| &line[range]),
                rewrite: &line[surface.rewrite],
            },
        },
        RuleLineSurfaceSpans::NeutralRewrite {
            application,
            rewrite,
        } => RuleLineSurface::NeutralRewrite {
            application: application.map(|application| application.application),
            rewrite: &line[rewrite],
        },
        RuleLineSurfaceSpans::OrientedRewrite {
            application,
            orientation,
            rewrite,
        } => RuleLineSurface::OrientedRewrite {
            application: application.map(|application| application.application),
            orientation: &line[orientation],
            rewrite: &line[rewrite],
        },
    })
}

pub fn rule_line_surface_spans(line: &str) -> Result<RuleLineSurfaceSpans, RuleLineSurfaceError> {
    let line_range = trimmed_range(line);
    let (application, rest) = split_rule_application_prefix_spans(line, line_range)?;
    if let Some(surface) =
        input_rewrite_surface_spans_in(line, rest.clone()).map_err(RuleLineSurfaceError::Input)?
    {
        return Ok(RuleLineSurfaceSpans::InputRewrite {
            application,
            surface,
        });
    }
    if line[rest.clone()].starts_with('[') {
        return Ok(RuleLineSurfaceSpans::NeutralRewrite {
            application,
            rewrite: rest,
        });
    }
    let Some(pattern_offset) = line[rest.clone()].find('[') else {
        return Err(RuleLineSurfaceError::MissingOrientation);
    };
    let rewrite_start = rest.start + pattern_offset;
    let orientation = trim_end_range(line, rest.start..rewrite_start);
    if orientation.is_empty() {
        return Err(RuleLineSurfaceError::MissingOrientation);
    }
    let rewrite = rewrite_start..rest.end;
    if !line[rewrite.clone()].starts_with('[') {
        return Err(RuleLineSurfaceError::RewriteMustStartWithPattern);
    }
    Ok(RuleLineSurfaceSpans::OrientedRewrite {
        application,
        orientation,
        rewrite,
    })
}

pub fn rule_line_semantic_surface_spans(
    line: &str,
) -> Result<Vec<RuleSemanticSurfaceSpan>, RuleLineSurfaceError> {
    let mut spans = Vec::new();
    match rule_line_surface_spans(line)? {
        RuleLineSurfaceSpans::InputRewrite { surface, .. } => {
            add_rule_rewrite_semantic_surface_spans(line, surface.rewrite, &mut spans);
        }
        RuleLineSurfaceSpans::NeutralRewrite { rewrite, .. }
        | RuleLineSurfaceSpans::OrientedRewrite { rewrite, .. } => {
            add_rule_rewrite_semantic_surface_spans(line, rewrite, &mut spans);
        }
    }
    Ok(spans)
}

fn add_rule_rewrite_semantic_surface_spans(
    line: &str,
    rewrite: Range<usize>,
    spans: &mut Vec<RuleSemanticSurfaceSpan>,
) {
    for cell in bracket_content_spans(line, rewrite) {
        let Ok(tokens) = cell_token_spans(line, cell) else {
            continue;
        };
        for token in tokens {
            add_rule_cell_token_semantic_surface_spans(line, token, spans);
        }
    }
}

fn bracket_content_spans(line: &str, range: Range<usize>) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    let mut open = None::<usize>;
    for (offset, ch) in line[range.clone()].char_indices() {
        let index = range.start + offset;
        match ch {
            '[' if open.is_none() => open = Some(index + ch.len_utf8()),
            ']' => {
                if let Some(start) = open.take() {
                    spans.push(start..index);
                }
            }
            _ => {}
        }
    }
    spans
}

fn cell_token_spans(line: &str, range: Range<usize>) -> Result<Vec<Range<usize>>, CellTokenError> {
    let mut spans = Vec::new();
    let mut token_start = None::<usize>;
    let mut brace_depth = 0_u16;
    let mut paren_depth = 0_u16;
    for (offset, ch) in line[range.clone()].char_indices() {
        let index = range.start + offset;
        match ch {
            ch if ch.is_whitespace() && brace_depth == 0 && paren_depth == 0 => {
                if let Some(start) = token_start.take() {
                    spans.push(start..index);
                }
            }
            '{' => {
                token_start.get_or_insert(index);
                brace_depth += 1;
            }
            '}' => {
                if brace_depth == 0 {
                    return Err(CellTokenError::UnmatchedCloseBrace);
                }
                token_start.get_or_insert(index);
                brace_depth -= 1;
            }
            '(' => {
                token_start.get_or_insert(index);
                paren_depth += 1;
            }
            ')' => {
                if paren_depth == 0 {
                    return Err(CellTokenError::UnmatchedCloseParen);
                }
                token_start.get_or_insert(index);
                paren_depth -= 1;
            }
            _ => {
                token_start.get_or_insert(index);
            }
        }
    }
    if brace_depth != 0 {
        return Err(CellTokenError::MissingCloseBrace);
    }
    if paren_depth != 0 {
        return Err(CellTokenError::MissingCloseParen);
    }
    if let Some(start) = token_start {
        spans.push(start..range.end);
    }
    Ok(spans)
}

fn add_rule_cell_token_semantic_surface_spans(
    line: &str,
    token: Range<usize>,
    spans: &mut Vec<RuleSemanticSurfaceSpan>,
) {
    let text = &line[token.clone()];
    if text == "|" {
        return;
    }
    if text == "no" {
        spans.push(RuleSemanticSurfaceSpan {
            kind: RuleSemanticSurfaceKind::Keyword,
            span: token,
        });
        return;
    }
    if mark_sugar_kind(text) == Some(MarkSugarKind::Movement) {
        spans.push(RuleSemanticSurfaceSpan {
            kind: RuleSemanticSurfaceKind::Direction,
            span: token,
        });
        return;
    }
    let mark_start = text.find('{').map(|offset| token.start + offset);
    let selector_end = mark_start.unwrap_or(token.end);
    if selector_end > token.start {
        spans.push(RuleSemanticSurfaceSpan {
            kind: RuleSemanticSurfaceKind::Object,
            span: token.start..selector_end,
        });
    }
    if let Some(open) = mark_start
        && line[token.clone()].ends_with('}')
    {
        spans.push(RuleSemanticSurfaceSpan {
            kind: RuleSemanticSurfaceKind::Mark,
            span: open..open + 1,
        });
        add_rule_mark_block_semantic_surface_spans(line, open + 1..token.end - 1, spans);
        spans.push(RuleSemanticSurfaceSpan {
            kind: RuleSemanticSurfaceKind::Mark,
            span: token.end - 1..token.end,
        });
    }
}

fn add_rule_mark_block_semantic_surface_spans(
    line: &str,
    range: Range<usize>,
    spans: &mut Vec<RuleSemanticSurfaceSpan>,
) {
    let Ok(tokens) = cell_token_spans(line, range) else {
        return;
    };
    for token in tokens {
        let text = &line[token.clone()];
        if text == "no" {
            spans.push(RuleSemanticSurfaceSpan {
                kind: RuleSemanticSurfaceKind::Keyword,
                span: token,
            });
            continue;
        }
        let end = text
            .find('=')
            .map_or(token.end, |offset| token.start + offset);
        spans.push(RuleSemanticSurfaceSpan {
            kind: RuleSemanticSurfaceKind::Mark,
            span: token.start..end,
        });
    }
}

fn split_rule_application_prefix_spans(
    line: &str,
    range: Range<usize>,
) -> Result<(Option<RuleApplicationSurfaceSpan>, Range<usize>), RuleLineSurfaceError> {
    let tokens = header_token_spans(line, range.clone());
    let Some(first) = tokens.first() else {
        return Ok((None, range));
    };
    let Some(application) = rule_application_surface(first.text) else {
        return Ok((None, range));
    };
    let rest = trim_start_range(line, first.range.end..range.end);
    if rest.is_empty() {
        return Err(RuleLineSurfaceError::MissingOrientation);
    }
    Ok((
        Some(RuleApplicationSurfaceSpan {
            application,
            span: first.range.clone(),
        }),
        rest,
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputRewriteSurface<'a> {
    pub orientation: Option<&'a str>,
    pub rewrite: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputRewriteSurfaceError {
    MissingRewrite,
    RewriteMustStartWithPattern,
}

impl InputRewriteSurfaceError {
    pub fn message(self) -> &'static str {
        match self {
            Self::MissingRewrite => "input rule must be: input <orientation> [ ... ] -> [ ... ]",
            Self::RewriteMustStartWithPattern => {
                "input rule must be: input <orientation> [ ... ] -> [ ... ]"
            }
        }
    }
}

pub fn input_rewrite_surface(
    line: &str,
) -> Result<Option<InputRewriteSurface<'_>>, InputRewriteSurfaceError> {
    Ok(
        input_rewrite_surface_spans(line)?.map(|surface| InputRewriteSurface {
            orientation: surface.orientation.map(|range| &line[range]),
            rewrite: &line[surface.rewrite],
        }),
    )
}

pub fn input_rewrite_surface_spans(
    line: &str,
) -> Result<Option<InputRewriteSurfaceSpans>, InputRewriteSurfaceError> {
    input_rewrite_surface_spans_in(line, trimmed_range(line))
}

fn input_rewrite_surface_spans_in(
    line: &str,
    range: Range<usize>,
) -> Result<Option<InputRewriteSurfaceSpans>, InputRewriteSurfaceError> {
    let tokens = header_token_spans(line, range.clone());
    let Some(first) = tokens.first() else {
        return Ok(None);
    };
    if first.text != "input" {
        return Ok(None);
    }
    let rest = trim_start_range(line, first.range.end..range.end);
    if rest.is_empty() {
        return Err(InputRewriteSurfaceError::MissingRewrite);
    }
    if line[rest.clone()].starts_with('[') {
        return Ok(Some(InputRewriteSurfaceSpans {
            input: first.range.clone(),
            orientation: None,
            rewrite: rest,
        }));
    }

    let rest_tokens = header_token_spans(line, rest.clone());
    let Some(orientation) = rest_tokens.first() else {
        return Err(InputRewriteSurfaceError::MissingRewrite);
    };
    let rewrite = trim_start_range(line, orientation.range.end..range.end);
    if !line[rewrite.clone()].starts_with('[') {
        return Err(InputRewriteSurfaceError::RewriteMustStartWithPattern);
    }
    Ok(Some(InputRewriteSurfaceSpans {
        input: first.range.clone(),
        orientation: Some(orientation.range.clone()),
        rewrite,
    }))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputOrientedPatternSurface {
    pub input: Range<usize>,
    pub orientation: Option<Range<usize>>,
}

pub fn input_oriented_pattern_surfaces(line: &str) -> Vec<InputOrientedPatternSurface> {
    let mut surfaces = Vec::new();
    let mut search_start = 0usize;
    while let Some(offset) = line[search_start..].find("input") {
        let input_start = search_start + offset;
        let input_end = input_start + "input".len();
        if !word_boundary_before(line, input_start) || !word_boundary_after(line, input_end) {
            search_start = input_end;
            continue;
        }
        if let Ok(Some(surface)) = input_rewrite_surface_spans_in(line, input_start..line.len()) {
            surfaces.push(InputOrientedPatternSurface {
                input: surface.input,
                orientation: surface.orientation,
            });
        }
        search_start = input_end;
    }
    surfaces
}

pub fn collect_rule_statement_line(lines: &[String], start: usize) -> (String, usize) {
    let first = lines[start].trim();
    if !looks_like_multiline_rule_line_start(first) {
        return (first.to_string(), start + 1);
    }

    let mut joined = String::new();
    let mut bracket_depth = 0usize;
    let mut saw_arrow = false;
    let mut index = start;
    while index < lines.len() {
        let line = lines[index].trim();
        if line == "}" {
            break;
        }
        if index > start && bracket_depth == 0 && !saw_arrow && !line.starts_with("->") {
            return (first.to_string(), start + 1);
        }
        if !joined.is_empty() {
            if bracket_depth > 0 {
                if line.starts_with('|') || joined.trim_end().ends_with('|') {
                    joined.push(' ');
                } else {
                    joined.push_str(" ; ");
                }
            } else {
                joined.push(' ');
            }
        }
        joined.push_str(line);
        bracket_depth = update_square_bracket_depth(bracket_depth, line);
        saw_arrow |= line.contains("->");

        if index == start && bracket_depth == 0 {
            return (first.to_string(), start + 1);
        }
        if index > start && bracket_depth == 0 && saw_arrow {
            return (joined, index + 1);
        }
        index += 1;
    }

    (first.to_string(), start + 1)
}

fn collect_rule_statement_entry_body(
    lines: &[String],
    block_name: &'static str,
) -> Result<Vec<RuleStatementSyntax>, RuleProgramBlockBodyError> {
    collect_rule_statement_body(lines, 0, block_name, false).map(|(body, _)| body)
}

fn collect_rule_statement_body(
    lines: &[String],
    mut index: usize,
    block_name: &'static str,
    closing_brace_required: bool,
) -> Result<(Vec<RuleStatementSyntax>, usize), RuleProgramBlockBodyError> {
    let mut body = Vec::new();
    while index < lines.len() {
        let line = lines[index].trim();
        if line == "}" {
            return Ok((body, index + 1));
        }
        if line.is_empty() {
            index += 1;
            continue;
        }
        if rule_statement_block_surface(line, true).is_some() {
            let header = line
                .strip_suffix('{')
                .expect("rule statement block surface requires an opening brace")
                .trim_end()
                .to_string();
            let (statements, next_index) =
                collect_rule_statement_body(lines, index + 1, block_name, true)?;
            body.push(RuleStatementSyntax::Block { header, statements });
            index = next_index;
            continue;
        }
        let (rule_line, next_index) = collect_rule_statement_line(lines, index);
        body.push(RuleStatementSyntax::Line(rule_line));
        index = next_index;
    }
    if closing_brace_required {
        Err(RuleProgramBlockBodyError::MissingClosingBrace { block_name })
    } else {
        Ok((body, index))
    }
}

fn collect_lifecycle_command_entry_body(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

fn looks_like_multiline_rule_line_start(line: &str) -> bool {
    line.contains('[')
        && (line.starts_with("input ")
            || line
                .split_once(' ')
                .is_some_and(|(prefix, _)| !prefix.is_empty()))
}

fn update_square_bracket_depth(mut depth: usize, line: &str) -> usize {
    let mut in_string = false;
    let mut escaped = false;
    for ch in line.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}

#[derive(Clone, Debug)]
struct HeaderTokenSpan<'a> {
    text: &'a str,
    range: Range<usize>,
}

fn header_token_spans(line: &str, range: Range<usize>) -> Vec<HeaderTokenSpan<'_>> {
    let mut tokens = Vec::new();
    let mut index = range.start;
    while index < range.end {
        let Some(start_offset) = line[index..range.end].find(|ch: char| !ch.is_whitespace()) else {
            break;
        };
        let start = index + start_offset;
        let end = line[start..range.end]
            .find(char::is_whitespace)
            .map_or(range.end, |offset| start + offset);
        tokens.push(HeaderTokenSpan {
            text: &line[start..end],
            range: start..end,
        });
        index = end;
    }
    if tokens.len() > 1 && tokens.last().is_some_and(|token| token.text == "{") {
        tokens.pop();
    }
    tokens
}

fn trimmed_range(line: &str) -> Range<usize> {
    let start = line
        .find(|ch: char| !ch.is_whitespace())
        .unwrap_or(line.len());
    let end = line
        .rfind(|ch: char| !ch.is_whitespace())
        .map(|index| {
            index
                + line[index..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(0)
        })
        .unwrap_or(start);
    start..end
}

fn trim_start_range(line: &str, range: Range<usize>) -> Range<usize> {
    let start = line[range.clone()]
        .find(|ch: char| !ch.is_whitespace())
        .map_or(range.end, |offset| range.start + offset);
    start..range.end
}

fn trim_end_range(line: &str, range: Range<usize>) -> Range<usize> {
    let end = line[range.clone()]
        .rfind(|ch: char| !ch.is_whitespace())
        .map(|offset| {
            let index = range.start + offset;
            index
                + line[index..]
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(0)
        })
        .unwrap_or(range.start);
    range.start..end
}

fn word_boundary_before(value: &str, index: usize) -> bool {
    value[..index]
        .chars()
        .next_back()
        .is_none_or(|ch| !is_word_continue(ch))
}

fn word_boundary_after(value: &str, index: usize) -> bool {
    value[index..]
        .chars()
        .next()
        .is_none_or(|ch| !is_word_continue(ch))
}

fn is_word_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkSugarKind {
    Movement,
    Bool,
    Int,
}

pub const ANONYMOUS_MOVEMENT_MARK_INDEX: u16 = 0;
pub const MOVEMENT_DIRECTIONS_2D: &[&str] = &["up", "down", "left", "right"];
pub const MOVEMENT_DIRECTIONS_3D: &[&str] = &["up", "down", "left", "right", "front", "back"];

pub fn mark_sugar_kind(token: &str) -> Option<MarkSugarKind> {
    if matches!(
        token,
        ">" | "<"
            | "^"
            | "v"
            | "up"
            | "down"
            | "left"
            | "right"
            | "front"
            | "back"
            | "forward"
            | "backward"
            | "directions"
            | "horizontal"
            | "vertical"
            | "parallel"
            | "perpendicular"
    ) {
        Some(MarkSugarKind::Movement)
    } else if matches!(token, "true" | "false") {
        Some(MarkSugarKind::Bool)
    } else if token.parse::<i64>().is_ok() {
        Some(MarkSugarKind::Int)
    } else {
        None
    }
}

pub fn canonical_3d_movement_direction_name(value: &str) -> &str {
    match value {
        "forward" => "front",
        "backward" => "back",
        other => other,
    }
}

pub fn movement_mark_index(value: &str, directions: &[&str]) -> Option<u16> {
    directions
        .iter()
        .position(|direction| *direction == value)
        .and_then(|index| u16::try_from(index).ok())
}

pub fn movement_mark_index_3d(value: &str) -> Option<u16> {
    movement_mark_index(
        canonical_3d_movement_direction_name(value),
        MOVEMENT_DIRECTIONS_3D,
    )
}

pub fn movement_mark_set_values(value: &str, dimensions: u8) -> Option<&'static [&'static str]> {
    match (value, dimensions) {
        ("directions", 2) => Some(MOVEMENT_DIRECTIONS_2D),
        ("directions", 3) => Some(MOVEMENT_DIRECTIONS_3D),
        ("horizontal", 2) => Some(&["left", "right"]),
        ("horizontal", 3) => Some(&["left", "right", "front", "back"]),
        ("vertical", 2) => Some(&["up", "down"]),
        ("vertical", 3) => Some(&["up", "down"]),
        ("parallel", 2) => Some(&["<", ">"]),
        ("perpendicular", 2) => Some(&["^", "v"]),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellTokenError {
    UnmatchedCloseBrace,
    MissingCloseBrace,
    UnmatchedCloseParen,
    MissingCloseParen,
}

/// Catalog-independent syntax shared by every spatial rule lowerer.
///
/// A semicolon is exactly a source newline. Consecutive separators therefore
/// produce `Blank` lines; dimensional lowering may assign spatial meaning to
/// those blank lines, but parsing them is part of the common language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnresolvedPatternSyntax {
    pub components: Vec<UnresolvedPatternComponentSyntax>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnresolvedPatternComponentSyntax {
    pub lines: Vec<UnresolvedPatternLineSyntax>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnresolvedPatternLineSyntax {
    Cells(Vec<UnresolvedPatternPartSyntax>),
    Blank,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnresolvedPatternPartSyntax {
    Cell(UnresolvedCellSyntax),
    Ellipsis,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnresolvedCellSyntax {
    pub keep: bool,
    pub require_null: bool,
    pub require: Vec<UnresolvedCellSubjectSyntax>,
    pub forbid: Vec<UnresolvedCellSubjectSyntax>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnresolvedCellSubjectSyntax {
    Selector(SelectorSyntax),
    CellMarks(Vec<SelectorMark>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnresolvedRewriteSyntax {
    pub before: UnresolvedPatternSyntax,
    pub after: Option<UnresolvedPatternSyntax>,
    pub suffix: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnresolvedPatternSyntaxError {
    RewriteMissingArrow,
    PatternMustStartWithBlock,
    PatternBlockMissingClose,
    PatternMustContainBlock,
    EmptyLine,
    KeepOnlyOnAfter,
    KeepWithOtherTokens,
    RewriteLayoutMismatch,
    CellPattern(CellPatternError),
    Selector(SelectorSyntaxError),
}

impl UnresolvedPatternSyntaxError {
    pub const fn message(&self) -> &'static str {
        match self {
            Self::RewriteMissingArrow => "inline rewrite must contain ->",
            Self::PatternMustStartWithBlock => "pattern side must contain bracketed blocks",
            Self::PatternBlockMissingClose => "pattern block missing ]",
            Self::PatternMustContainBlock => "pattern side must contain at least one block",
            Self::EmptyLine => "pattern line must contain at least one cell",
            Self::KeepOnlyOnAfter => "`=` is only valid as a RHS cell",
            Self::KeepWithOtherTokens => "`=` RHS cell cannot contain other tokens",
            Self::RewriteLayoutMismatch => {
                "before and after blocks must have matching cell, ellipsis, and blank-line layout"
            }
            Self::CellPattern(error) => error.message(),
            Self::Selector(error) => error.message(),
        }
    }
}

pub fn parse_unresolved_rewrite_syntax(
    source: &str,
) -> Result<UnresolvedRewriteSyntax, UnresolvedPatternSyntaxError> {
    let (before, after) = source
        .split_once("->")
        .ok_or(UnresolvedPatternSyntaxError::RewriteMissingArrow)?;
    let (before, before_suffix) = parse_unresolved_pattern_prefix(before, false)?;
    if !before_suffix.is_empty() {
        return Err(UnresolvedPatternSyntaxError::PatternMustStartWithBlock);
    }
    let after = after.trim();
    if after.starts_with('[') {
        let (mut after, suffix) = parse_unresolved_pattern_prefix(after, true)?;
        normalize_unresolved_keep_cells(&before, &mut after)?;
        Ok(UnresolvedRewriteSyntax {
            before,
            after: Some(after),
            suffix: suffix.to_string(),
        })
    } else {
        Ok(UnresolvedRewriteSyntax {
            before,
            after: None,
            suffix: after.to_string(),
        })
    }
}

fn normalize_unresolved_keep_cells(
    before: &UnresolvedPatternSyntax,
    after: &mut UnresolvedPatternSyntax,
) -> Result<(), UnresolvedPatternSyntaxError> {
    for (component_index, after_component) in after.components.iter_mut().enumerate() {
        for (line_index, after_line) in after_component.lines.iter_mut().enumerate() {
            let UnresolvedPatternLineSyntax::Cells(after_parts) = after_line else {
                continue;
            };
            for (part_index, after_part) in after_parts.iter_mut().enumerate() {
                let UnresolvedPatternPartSyntax::Cell(after_cell) = after_part else {
                    continue;
                };
                if !after_cell.keep {
                    continue;
                }
                let Some(UnresolvedPatternPartSyntax::Cell(before_cell)) = before
                    .components
                    .get(component_index)
                    .and_then(|component| component.lines.get(line_index))
                    .and_then(|line| match line {
                        UnresolvedPatternLineSyntax::Cells(parts) => parts.get(part_index),
                        UnresolvedPatternLineSyntax::Blank => None,
                    })
                else {
                    return Err(UnresolvedPatternSyntaxError::RewriteLayoutMismatch);
                };
                *after_cell = before_cell.clone();
            }
        }
    }
    Ok(())
}

pub fn parse_unresolved_pattern_syntax(
    source: &str,
) -> Result<UnresolvedPatternSyntax, UnresolvedPatternSyntaxError> {
    let (pattern, suffix) = parse_unresolved_pattern_prefix(source, false)?;
    if !suffix.is_empty() {
        return Err(UnresolvedPatternSyntaxError::PatternMustStartWithBlock);
    }
    Ok(pattern)
}

fn parse_unresolved_pattern_prefix(
    source: &str,
    allow_keep: bool,
) -> Result<(UnresolvedPatternSyntax, &str), UnresolvedPatternSyntaxError> {
    let mut components = Vec::new();
    let mut rest = source.trim();
    while let Some(inner) = rest.strip_prefix('[') {
        let close = inner
            .find(']')
            .ok_or(UnresolvedPatternSyntaxError::PatternBlockMissingClose)?;
        components.push(parse_unresolved_pattern_component(
            &inner[..close],
            allow_keep,
        )?);
        rest = inner[close + 1..].trim_start();
    }
    if components.is_empty() {
        return Err(UnresolvedPatternSyntaxError::PatternMustContainBlock);
    }
    Ok((UnresolvedPatternSyntax { components }, rest.trim()))
}

fn parse_unresolved_pattern_component(
    source: &str,
    allow_keep: bool,
) -> Result<UnresolvedPatternComponentSyntax, UnresolvedPatternSyntaxError> {
    let source = source.trim();
    if source.is_empty() {
        return Ok(UnresolvedPatternComponentSyntax {
            lines: vec![UnresolvedPatternLineSyntax::Cells(vec![
                UnresolvedPatternPartSyntax::Cell(UnresolvedCellSyntax::default()),
            ])],
        });
    }
    let mut lines = Vec::new();
    for line in source.split([';', '\n']) {
        let line = line.trim().trim_end_matches('\r').trim();
        if line.is_empty() {
            lines.push(UnresolvedPatternLineSyntax::Blank);
            continue;
        }
        let parts = line
            .split('|')
            .map(str::trim)
            .map(|cell| {
                if cell == "..." {
                    Ok(UnresolvedPatternPartSyntax::Ellipsis)
                } else {
                    parse_unresolved_cell_syntax(cell, allow_keep)
                        .map(UnresolvedPatternPartSyntax::Cell)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        if parts.is_empty() {
            return Err(UnresolvedPatternSyntaxError::EmptyLine);
        }
        lines.push(UnresolvedPatternLineSyntax::Cells(parts));
    }
    if lines.is_empty() {
        return Err(UnresolvedPatternSyntaxError::EmptyLine);
    }
    Ok(UnresolvedPatternComponentSyntax { lines })
}

fn parse_unresolved_cell_syntax(
    source: &str,
    allow_keep: bool,
) -> Result<UnresolvedCellSyntax, UnresolvedPatternSyntaxError> {
    let tokens = split_cell_tokens(source)
        .map_err(CellPatternError::Token)
        .map_err(UnresolvedPatternSyntaxError::CellPattern)?;
    if tokens.iter().any(|token| token == "=") {
        if !allow_keep {
            return Err(UnresolvedPatternSyntaxError::KeepOnlyOnAfter);
        }
        if tokens.len() != 1 {
            return Err(UnresolvedPatternSyntaxError::KeepWithOtherTokens);
        }
        return Ok(UnresolvedCellSyntax {
            keep: true,
            ..UnresolvedCellSyntax::default()
        });
    }

    let mut cell = UnresolvedCellSyntax::default();
    let mut index = 0;
    while index < tokens.len() {
        let token = &tokens[index];
        if token == "no" {
            let subject = tokens
                .get(index + 1)
                .ok_or(CellPatternError::MissingForbiddenSelector)
                .map_err(UnresolvedPatternSyntaxError::CellPattern)?;
            if subject == "no" {
                return Err(UnresolvedPatternSyntaxError::CellPattern(
                    CellPatternError::RepeatedNo,
                ));
            }
            if subject == "null" {
                return Err(UnresolvedPatternSyntaxError::CellPattern(
                    CellPatternError::ForbidNull,
                ));
            }
            cell.forbid.push(parse_unresolved_cell_subject(subject)?);
            index += 2;
            continue;
        }
        if mark_sugar_kind(token).is_some() {
            let subject = tokens
                .get(index + 1)
                .ok_or(CellPatternError::MissingMarkSugarSelector)
                .map_err(UnresolvedPatternSyntaxError::CellPattern)?;
            if subject == "no" || mark_sugar_kind(subject).is_some() {
                return Err(UnresolvedPatternSyntaxError::CellPattern(
                    CellPatternError::InvalidMarkSugarSelector,
                ));
            }
            if subject == "null" {
                return Err(UnresolvedPatternSyntaxError::CellPattern(
                    CellPatternError::NullMixedWithOtherTokens,
                ));
            }
            let mut selector =
                parse_selector_syntax(subject).map_err(UnresolvedPatternSyntaxError::Selector)?;
            selector.marks.push(SelectorMark {
                name: String::new(),
                value: Some(token.clone()),
                negated: false,
            });
            cell.require
                .push(UnresolvedCellSubjectSyntax::Selector(selector));
            index += 2;
            continue;
        }
        if token == "null" {
            if tokens.len() != 1 {
                return Err(UnresolvedPatternSyntaxError::CellPattern(
                    CellPatternError::NullMixedWithOtherTokens,
                ));
            }
            cell.require_null = true;
        } else {
            cell.require.push(parse_unresolved_cell_subject(token)?);
        }
        index += 1;
    }
    Ok(cell)
}

fn parse_unresolved_cell_subject(
    source: &str,
) -> Result<UnresolvedCellSubjectSyntax, UnresolvedPatternSyntaxError> {
    if source.starts_with('{') {
        let synthetic = format!("__cell{source}");
        let syntax =
            parse_selector_syntax(&synthetic).map_err(UnresolvedPatternSyntaxError::Selector)?;
        return Ok(UnresolvedCellSubjectSyntax::CellMarks(syntax.marks));
    }
    parse_selector_syntax(source)
        .map(UnresolvedCellSubjectSyntax::Selector)
        .map_err(UnresolvedPatternSyntaxError::Selector)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellPatternError {
    Token(CellTokenError),
    MissingForbiddenSelector,
    RepeatedNo,
    ForbidNull,
    MissingMarkSugarSelector,
    InvalidMarkSugarSelector,
    NullMixedWithOtherTokens,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NullCellPatternError {
    IntroducedOnRewrite,
    WriteToNull,
    MissingAnchor,
}

impl NullCellPatternError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::IntroducedOnRewrite => {
                "`null` can only be matched on the before side of a rewrite"
            }
            Self::WriteToNull => "`null` matched cells cannot be written to",
            Self::MissingAnchor => "`null` patterns must include at least one non-null cell",
        }
    }
}

pub fn validate_null_pattern_cells(
    require_null: impl IntoIterator<Item = bool>,
) -> Result<(), NullCellPatternError> {
    let mut has_null = false;
    let mut has_non_null = false;
    for require_null in require_null {
        has_null |= require_null;
        has_non_null |= !require_null;
    }
    if has_null && !has_non_null {
        return Err(NullCellPatternError::MissingAnchor);
    }
    Ok(())
}

pub fn validate_null_rewrite_cell(
    before_null: bool,
    after_null: bool,
    after_empty: bool,
) -> Result<(), NullCellPatternError> {
    if after_null && !before_null {
        return Err(NullCellPatternError::IntroducedOnRewrite);
    }
    if before_null && !after_empty {
        return Err(NullCellPatternError::WriteToNull);
    }
    Ok(())
}

impl CellPatternError {
    pub const fn message(&self) -> &'static str {
        match self {
            Self::Token(CellTokenError::UnmatchedCloseBrace) => "mark block has unmatched }",
            Self::Token(CellTokenError::MissingCloseBrace) => "mark block missing }",
            Self::Token(CellTokenError::UnmatchedCloseParen) => "cell selector has unmatched )",
            Self::Token(CellTokenError::MissingCloseParen) => "cell selector missing )",
            Self::MissingForbiddenSelector => "`no` must be followed by a selector",
            Self::RepeatedNo => "`no no` is not a valid cell pattern",
            Self::ForbidNull => "`no null` is not a valid cell pattern",
            Self::MissingMarkSugarSelector | Self::InvalidMarkSugarSelector => {
                "mark sugar must be followed by a selector"
            }
            Self::NullMixedWithOtherTokens => "`null` cell pattern cannot contain other tokens",
        }
    }
}

pub fn split_cell_tokens(cell: &str) -> Result<Vec<String>, CellTokenError> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut brace_depth = 0_u16;
    let mut paren_depth = 0_u16;
    for ch in cell.chars() {
        match ch {
            '{' => {
                brace_depth += 1;
                token.push(ch);
            }
            '}' => {
                if brace_depth == 0 {
                    return Err(CellTokenError::UnmatchedCloseBrace);
                }
                brace_depth -= 1;
                token.push(ch);
            }
            '(' => {
                paren_depth += 1;
                token.push(ch);
            }
            ')' => {
                if paren_depth == 0 {
                    return Err(CellTokenError::UnmatchedCloseParen);
                }
                paren_depth -= 1;
                token.push(ch);
            }
            ch if ch.is_whitespace() && brace_depth == 0 && paren_depth == 0 => {
                if !token.is_empty() {
                    tokens.push(std::mem::take(&mut token));
                }
            }
            _ => token.push(ch),
        }
    }
    if brace_depth != 0 {
        return Err(CellTokenError::MissingCloseBrace);
    }
    if paren_depth != 0 {
        return Err(CellTokenError::MissingCloseParen);
    }
    if !token.is_empty() {
        tokens.push(token);
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_syntax_is_dimension_independent() {
        assert_eq!(
            parse_selector_syntax("TEN#moving:>{right no active}").unwrap(),
            SelectorSyntax {
                selector: "TEN:>".to_string(),
                base: "TEN".to_string(),
                tags: vec![">".to_string()],
                occurrence_label: Some("moving".to_string()),
                marks: vec![
                    SelectorMark {
                        name: String::new(),
                        value: Some("right".to_string()),
                        negated: false,
                    },
                    SelectorMark {
                        name: "active".to_string(),
                        value: None,
                        negated: true,
                    },
                ],
            }
        );
    }

    #[test]
    fn pattern_lines_treat_semicolon_as_newline_and_preserve_blank_lines() {
        let semicolons = parse_unresolved_pattern_syntax("[A;B;;C]").unwrap();
        let newlines = parse_unresolved_pattern_syntax("[A\nB\n\nC]").unwrap();
        assert_eq!(semicolons, newlines);
        assert!(matches!(
            semicolons.components[0].lines[2],
            UnresolvedPatternLineSyntax::Blank
        ));
    }

    #[test]
    fn unresolved_rewrite_owns_cells_selectors_and_keep_normalization() {
        let rewrite = parse_unresolved_rewrite_syntax(
            "[TEN#moving:>{right} | no Wall] -> [= | TEN#moving:>] camera_follow",
        )
        .unwrap();
        assert_eq!(rewrite.suffix, "camera_follow");
        let after = rewrite.after.unwrap();
        let UnresolvedPatternLineSyntax::Cells(parts) = &after.components[0].lines[0] else {
            panic!("expected cell line");
        };
        let UnresolvedPatternLineSyntax::Cells(before_parts) =
            &rewrite.before.components[0].lines[0]
        else {
            panic!("expected before cell line");
        };
        assert_eq!(parts[0], before_parts[0]);
    }

    #[test]
    fn split_header_tokens_keeps_parenthesized_values_together() {
        assert_eq!(
            split_header_tokens("legend b = Ball:(0, 1):red"),
            ["legend", "b", "=", "Ball:(0, 1):red"]
        );
    }

    #[test]
    fn new_puzzle_source_is_blank() {
        let source = new_puzzle_source("Custom Puzzle");

        assert_eq!(source, "");
        assert!(!source.contains("title "));
        assert!(!source.contains("puzzle main {"));
        assert!(!source.contains("levels "));
        assert!(!source.contains("keys {"));
        assert!(!source.contains("slots"));
        assert!(!source.contains("base ="));
        assert!(!source.contains("floor ="));
        assert!(!source.contains("solid ="));
        assert!(!source.contains("scene title {"));
        assert!(!source.contains("scene level_select {"));
        assert!(!source.contains("scene playing {"));
        assert!(!source.contains("<-"));
        assert!(!source.contains("inputs {"));
        assert!(!source.contains("exists("));
        assert!(!source.contains("none("));
        assert!(!source.contains("input directions"));
    }

    #[test]
    fn recognizes_at_prefixed_identifiers_without_assigning_a_role() {
        assert!(is_at_identifier_token("@Trail"));
        assert!(is_at_identifier_token("@Trail:kind"));
        assert!(is_at_identifier_token("@Trail{right}"));
        assert!(!is_at_identifier_token("Trail"));
        assert!(!is_at_identifier_token("@"));
        assert!(!is_at_identifier_token("@:kind"));
    }

    #[test]
    fn shared_mark_sugar_recognizes_2d_and_3d_direction_words() {
        assert_eq!(mark_sugar_kind(">"), Some(MarkSugarKind::Movement));
        assert_eq!(mark_sugar_kind("front"), Some(MarkSugarKind::Movement));
        assert_eq!(mark_sugar_kind("true"), Some(MarkSugarKind::Bool));
        assert_eq!(mark_sugar_kind("7"), Some(MarkSugarKind::Int));
        assert_eq!(mark_sugar_kind("Player"), None);
    }

    #[test]
    fn shared_rule_surface_recognizes_common_input_rewrite_and_move_step() {
        assert_eq!(
            key_binding_surface("q Escape -> restart").unwrap(),
            KeyBindingSurface {
                keys: vec!["q", "Escape"],
                target: "restart",
            }
        );
        assert_eq!(
            key_binding_surface("q Escape = restart").unwrap_err(),
            KeyBindingSurfaceError::UseArrow
        );
        assert_eq!(
            key_binding_surface("-> restart").unwrap_err(),
            KeyBindingSurfaceError::MissingKeys
        );
        assert_eq!(
            key_binding_surface("q ->").unwrap_err(),
            KeyBindingSurfaceError::MissingTarget
        );
        assert_eq!(
            split_header_tokens("rules local_frame 3 full {"),
            vec!["rules", "local_frame", "3", "full"]
        );
        assert_eq!(
            rule_program_block_surface("rules local_frame 3 full {"),
            Some(RuleProgramBlockSurface::Rules {
                modifier: "local_frame 3 full"
            })
        );
        assert_eq!(
            rule_program_block_surface("on_level_start {"),
            Some(RuleProgramBlockSurface::OnLevelStart { modifier: "" })
        );
        assert_eq!(
            rule_program_block_surface("on_level_clear {"),
            Some(RuleProgramBlockSurface::OnLevelClear)
        );
        assert_eq!(
            rule_statement_block_surface("rules local_frame 3 full {", false),
            Some(RuleStatementBlockSurface::Program(
                RuleProgramBlockSurface::Rules {
                    modifier: "local_frame 3 full"
                }
            ))
        );
        assert_eq!(
            rule_statement_block_surface("routine slide once {", false),
            Some(RuleStatementBlockSurface::Routine)
        );
        let routine_spans = rule_routine_block_header_surface_spans("routine slide once {")
            .expect("routine header spans");
        assert_eq!(routine_spans.keyword, 0.."routine".len());
        assert_eq!(
            routine_spans.name,
            Some("routine ".len().."routine slide".len())
        );
        assert_eq!(
            routine_spans.modifiers,
            vec!["routine slide ".len().."routine slide once".len()]
        );
        assert_eq!(
            rule_statement_block_surface("if true {", true),
            Some(RuleStatementBlockSurface::Nested)
        );
        assert_eq!(
            rule_statement_block_surface("restart -> {", true),
            Some(RuleStatementBlockSurface::Nested)
        );
        assert_eq!(rule_statement_block_surface("render {", true), None);
        assert_eq!(
            rule_statement_surface("move").unwrap(),
            RuleStatementSurface::Call { name: "move" }
        );
        assert_eq!(
            rule_statement_surface("once {").unwrap(),
            RuleStatementSurface::ApplicationBlock {
                application: RuleApplicationSurface::Once
            }
        );
        assert_eq!(
            rule_statement_surface("push_boxes").unwrap(),
            RuleStatementSurface::Call { name: "push_boxes" }
        );
        assert_eq!(
            input_rewrite_surface("input [ Player ] -> [ > Player ]").unwrap(),
            Some(InputRewriteSurface {
                orientation: None,
                rewrite: "[ Player ] -> [ > Player ]",
            })
        );
        assert_eq!(
            rule_line_surface("input [ Player ] -> [ > Player ]").unwrap(),
            RuleLineSurface::InputRewrite {
                application: None,
                surface: InputRewriteSurface {
                    orientation: None,
                    rewrite: "[ Player ] -> [ > Player ]",
                },
            }
        );
        assert_eq!(
            rule_line_surface("once input [ Player ] -> [ > Player ]").unwrap(),
            RuleLineSurface::InputRewrite {
                application: Some(RuleApplicationSurface::Once),
                surface: InputRewriteSurface {
                    orientation: None,
                    rewrite: "[ Player ] -> [ > Player ]",
                },
            }
        );
        assert_eq!(
            rule_line_surface_spans("once input directions [ Player ] -> [ > Player ]").unwrap(),
            RuleLineSurfaceSpans::InputRewrite {
                application: Some(RuleApplicationSurfaceSpan {
                    application: RuleApplicationSurface::Once,
                    span: 0..4,
                }),
                surface: InputRewriteSurfaceSpans {
                    input: 5..10,
                    orientation: Some(11..21),
                    rewrite: 22..48,
                },
            }
        );
        assert_eq!(
            input_rewrite_surface("input horizontal [ Player ] -> [ > Player ]").unwrap(),
            Some(InputRewriteSurface {
                orientation: Some("horizontal"),
                rewrite: "[ Player ] -> [ > Player ]",
            })
        );
        assert_eq!(
            input_oriented_pattern_surfaces("if some(input directions [ Player | Wall ]) {"),
            vec![InputOrientedPatternSurface {
                input: 8..13,
                orientation: Some(14..24),
            }]
        );
        assert_eq!(
            input_oriented_pattern_surfaces("if some(input [ Player | Wall ]) {"),
            vec![InputOrientedPatternSurface {
                input: 8..13,
                orientation: None,
            }]
        );
        assert_eq!(
            input_oriented_pattern_surfaces("input directions [ Player | Wall ] -> push_player"),
            vec![InputOrientedPatternSurface {
                input: 0..5,
                orientation: Some(6..16),
            }]
        );
        assert_eq!(
            input_oriented_pattern_surfaces("input [ Player | Wall ] -> push_player"),
            vec![InputOrientedPatternSurface {
                input: 0..5,
                orientation: None,
            }]
        );
        assert_eq!(
            input_oriented_pattern_surfaces(
                "once input directions [ Player | Wall ] -> [ Player ]"
            ),
            vec![InputOrientedPatternSurface {
                input: 5..10,
                orientation: Some(11..21),
            }]
        );
        assert_eq!(
            input_oriented_pattern_surfaces("once input [ Player | Wall ] -> [ Player ]"),
            vec![InputOrientedPatternSurface {
                input: 5..10,
                orientation: None,
            }]
        );
        assert!(
            input_oriented_pattern_surfaces("input right").is_empty(),
            "scene/input commands without a following pattern are not input-oriented pattern surfaces"
        );
        let multiline = vec![
            "input directions [ Player".to_string(),
            "| no Wall ]".to_string(),
            "-> [".to_string(),
            "| Player ]".to_string(),
            "move".to_string(),
        ];
        assert_eq!(
            collect_rule_statement_line(&multiline, 0),
            (
                "input directions [ Player | no Wall ] -> [ | Player ]".to_string(),
                4,
            )
        );
        assert_eq!(
            collect_rule_statement_line(&multiline, 4),
            ("move".to_string(), 5)
        );
        let rule_program_lines = vec![
            "input directions [ Player".to_string(),
            "| no Wall ]".to_string(),
            "-> [".to_string(),
            "| Player ]".to_string(),
            "move".to_string(),
        ];
        assert_eq!(
            collect_rule_program_entry_body(
                &rule_program_lines,
                RuleProgramBlockSurface::Rules { modifier: "" },
            )
            .unwrap(),
            RuleProgramBlockBody::RuleStatements(vec![
                RuleStatementSyntax::Line(
                    "input directions [ Player | no Wall ] -> [ | Player ]".to_string(),
                ),
                RuleStatementSyntax::Line("move".to_string()),
            ])
        );
        let nested_rule_program_lines = vec![
            "for h in horizontal {".to_string(),
            "if input == h {".to_string(),
            "[ TEN:horizontal ] -> [ TEN:h ]".to_string(),
            "}".to_string(),
            "}".to_string(),
        ];
        assert_eq!(
            collect_rule_program_entry_body(
                &nested_rule_program_lines,
                RuleProgramBlockSurface::Rules { modifier: "" },
            )
            .unwrap(),
            RuleProgramBlockBody::RuleStatements(vec![RuleStatementSyntax::Block {
                header: "for h in horizontal".to_string(),
                statements: vec![RuleStatementSyntax::Block {
                    header: "if input == h".to_string(),
                    statements: vec![RuleStatementSyntax::Line(
                        "[ TEN:horizontal ] -> [ TEN:h ]".to_string(),
                    )],
                }],
            }])
        );
        let dense_multiline = vec![
            "(right, up) [ Player".to_string(),
            "Box ] -> [ Player".to_string(),
            "Box ]".to_string(),
        ];
        assert_eq!(
            collect_rule_statement_line(&dense_multiline, 0),
            (
                "(right, up) [ Player ; Box ] -> [ Player ; Box ]".to_string(),
                3,
            )
        );
        let lifecycle_lines = vec![
            "".to_string(),
            "if win_conditions -> next_level".to_string(),
        ];
        assert_eq!(
            collect_rule_program_entry_body(
                &lifecycle_lines,
                RuleProgramBlockSurface::OnLevelClear,
            )
            .unwrap(),
            RuleProgramBlockBody::LifecycleCommands(vec![
                "if win_conditions -> next_level".to_string()
            ])
        );
        assert_eq!(
            rule_line_surface("right [ Player ] -> [ > Player ]").unwrap(),
            RuleLineSurface::OrientedRewrite {
                application: None,
                orientation: "right",
                rewrite: "[ Player ] -> [ > Player ]",
            }
        );
        assert_eq!(
            rule_line_surface("repeat right [ Player ] -> [ > Player ]").unwrap(),
            RuleLineSurface::OrientedRewrite {
                application: Some(RuleApplicationSurface::Repeat),
                orientation: "right",
                rewrite: "[ Player ] -> [ > Player ]",
            }
        );
        assert_eq!(
            rule_line_surface("right, front [ Player ] -> [ Player ]").unwrap(),
            RuleLineSurface::OrientedRewrite {
                application: None,
                orientation: "right, front",
                rewrite: "[ Player ] -> [ Player ]",
            }
        );
        assert_eq!(
            rule_line_surface("(right, front) [ Player ] -> [ Player ]").unwrap(),
            RuleLineSurface::OrientedRewrite {
                application: None,
                orientation: "(right, front)",
                rewrite: "[ Player ] -> [ Player ]",
            }
        );
        assert_eq!(
            rule_line_surface("[ > Player | Box ] -> [ > Player | > Box ]").unwrap(),
            RuleLineSurface::NeutralRewrite {
                application: None,
                rewrite: "[ > Player | Box ] -> [ > Player | > Box ]",
            }
        );
        assert_eq!(
            rule_line_surface("once_all [ > Player | Box ] -> [ > Player | > Box ]").unwrap(),
            RuleLineSurface::NeutralRewrite {
                application: Some(RuleApplicationSurface::OnceAll),
                rewrite: "[ > Player | Box ] -> [ > Player | > Box ]",
            }
        );
    }

    #[test]
    fn shared_movement_contract_resolves_direction_aliases_and_sets() {
        assert_eq!(
            movement_mark_index("right", MOVEMENT_DIRECTIONS_2D),
            Some(3)
        );
        assert_eq!(movement_mark_index_3d("forward"), Some(4));
        assert_eq!(
            movement_mark_index("forward", MOVEMENT_DIRECTIONS_2D),
            None,
            "forward/backward aliases are 3D-specific"
        );
        assert_eq!(
            movement_mark_set_values("horizontal", 3),
            Some(["left", "right", "front", "back"].as_slice())
        );
        assert_eq!(
            movement_mark_set_values("perpendicular", 3),
            None,
            "relative 2D movement sets are not defined for 3D line space"
        );
    }

    #[test]
    fn shared_cell_tokenizer_keeps_mark_blocks_together() {
        assert_eq!(
            split_cell_tokens("Player{> no flag} no Wall").unwrap(),
            vec!["Player{> no flag}", "no", "Wall"]
        );
    }

    #[test]
    fn rule_semantic_surface_splits_compact_selector_marks() {
        let line = "[ > Player{mark} ] -> [ Player ]";
        let spans = rule_line_semantic_surface_spans(line).unwrap();
        let projected = spans
            .iter()
            .map(|span| (span.kind, &line[span.span.clone()]))
            .collect::<Vec<_>>();

        assert!(projected.contains(&(RuleSemanticSurfaceKind::Direction, ">")));
        assert!(projected.contains(&(RuleSemanticSurfaceKind::Object, "Player")));
        assert!(projected.contains(&(RuleSemanticSurfaceKind::Mark, "{")));
        assert!(projected.contains(&(RuleSemanticSurfaceKind::Mark, "mark")));
        assert!(projected.contains(&(RuleSemanticSurfaceKind::Mark, "}")));
    }

    #[test]
    fn shared_cell_tokenizer_keeps_computed_axis_selectors_together() {
        assert_eq!(
            split_cell_tokens("Box:(facing + 90deg):red no Wall").unwrap(),
            vec!["Box:(facing + 90deg):red", "no", "Wall"]
        );
    }

    #[test]
    fn shared_layer_rows_distinguish_anonymous_named_and_each_forms() {
        assert_eq!(
            slot_row_surface("Player Box"),
            Some(SlotRowSurface::Anonymous {
                selectors: vec!["Player", "Box"],
            })
        );
        assert_eq!(
            slot_row_surface("solid = Player Box"),
            Some(SlotRowSurface::Named(SelectorAssignmentSurface {
                name: "solid",
                selectors: vec!["Player", "Box"],
            }))
        );
        assert_eq!(
            slot_row_surface("each Player Box"),
            Some(SlotRowSurface::Each {
                selectors: vec!["Player", "Box"],
            })
        );
        assert_eq!(
            selector_assignment_surface("𝒞 = Crate Background"),
            Some(SelectorAssignmentSurface {
                name: "𝒞",
                selectors: vec!["Crate", "Background"],
            })
        );
        assert!(selector_alias_conflicts(
            "solid",
            ["Player"],
            ["Door"],
            ["solid"]
        ));
        assert!(!selector_alias_conflicts(
            "target",
            ["Player"],
            ["Door"],
            ["solid"]
        ));
    }

    #[test]
    fn shared_puzzle_directive_surface_classifies_dimension_independent_heads() {
        assert_eq!(
            puzzle_directive_surface("puzzle board {"),
            PuzzleDirectiveSurface::Model
        );
        assert_eq!(
            puzzle_directive_surface("puzzle3 board {"),
            PuzzleDirectiveSurface::Unknown
        );
        assert_eq!(
            puzzle_directive_surface("slots {"),
            PuzzleDirectiveSurface::Slots
        );
        assert_eq!(
            puzzle_directive_surface("map turn axis {"),
            PuzzleDirectiveSurface::Map
        );
        assert!(puzzle_directive_surface("map turn axis {").is_catalog_owned());
        assert_eq!(
            puzzle_directive_surface("on_level_start {"),
            PuzzleDirectiveSurface::RuleProgram
        );
        assert_eq!(
            puzzle_directive_surface("levels3 old {"),
            PuzzleDirectiveSurface::RemovedLevels3
        );
        assert_eq!(
            puzzle_directive_surface("state = open closed"),
            PuzzleDirectiveSurface::Assignment
        );
        assert_eq!(
            puzzle_directive_surface("theme = \"puzzlescript\""),
            PuzzleDirectiveSurface::Metadata
        );
        assert_eq!(
            puzzle_directive_surface("theme dark {"),
            PuzzleDirectiveSurface::DocumentShell
        );
    }

    #[test]
    fn shared_resource_headers_cover_named_and_owner_scoped_forms() {
        assert_eq!(
            resource_header_surface("levels {", "levels").unwrap(),
            ResourceHeaderSurface {
                name: None,
                owner: None,
            }
        );
        assert_eq!(
            resource_header_surface("levels microban of sokoban {", "levels").unwrap(),
            ResourceHeaderSurface {
                name: Some("microban"),
                owner: Some("sokoban"),
            }
        );
        assert_eq!(
            resource_header_surface("sprites of board {", "sprites").unwrap(),
            ResourceHeaderSurface {
                name: None,
                owner: Some("board"),
            }
        );
        assert_eq!(
            resource_header_surface("sprites demo", "sprites").unwrap(),
            ResourceHeaderSurface {
                name: Some("demo"),
                owner: None,
            }
        );
        assert!(resource_header_surface("levels bad name {", "levels").is_err());
        assert!(
            collect_resource_block_surface(&["sprites demo".to_string()], 0, "sprites").is_err()
        );

        let lines = [
            "sprites icons of board {",
            "shapes {",
            "dot {",
            "0",
            "}",
            "}",
            "Player {",
            "#fff",
            "0",
            "}",
            "}",
            "after",
        ]
        .map(str::to_string);
        assert_eq!(
            collect_resource_block_surface(&lines, 0, "sprites").unwrap(),
            ResourceBlockSurface {
                header: ResourceHeaderSurface {
                    name: Some("icons"),
                    owner: Some("board"),
                },
                body_start: 1,
                body_end: 10,
                next_index: 11,
            }
        );
    }

    #[test]
    fn shared_row_block_collector_owns_close_and_nested_block_rules() {
        let lines = ["A", "", "B", "}", "after"].map(str::to_string);
        assert_eq!(
            collect_row_block_surface(&lines, 0, "groups").unwrap(),
            RowBlockSurface {
                rows: vec!["A", "B"],
                next_index: 4,
            }
        );
        let nested = ["child {", "}", "}"].map(str::to_string);
        assert!(
            collect_row_block_surface(&nested, 0, "groups")
                .unwrap_err()
                .message()
                .contains("not nested blocks")
        );
        let container = ["child {", "row", "}", "}", "after"].map(str::to_string);
        assert_eq!(
            collect_container_block_surface(&container, 0, "levels").unwrap(),
            ContainerBlockSurface {
                body_start: 0,
                body_end: 3,
                next_index: 4,
            }
        );
    }

    #[test]
    fn shared_win_condition_rows_normalize_function_and_legacy_forms() {
        assert_eq!(
            win_condition_row_surface("exists([ Player | Goal ])").unwrap(),
            WinConditionRowSurface::Query {
                quantifier: WinConditionQuantifier::Exists,
                argument: "[ Player | Goal ]",
            }
        );
        assert_eq!(
            win_condition_row_surface("no Box").unwrap(),
            WinConditionRowSurface::Query {
                quantifier: WinConditionQuantifier::None,
                argument: "Box",
            }
        );
        assert_eq!(
            win_condition_row_surface("all Box on Goal").unwrap(),
            WinConditionRowSurface::AllOn {
                subject: "Box",
                cover: "Goal",
            }
        );
        assert_eq!(
            win_condition_row_surface("score >= 10").unwrap(),
            WinConditionRowSurface::Expression("score >= 10")
        );
    }

    #[test]
    fn shared_layer_selector_expansion_resolves_forward_groups_and_cycles() {
        let groups = vec![
            SelectorGroupDeclaration {
                name: "solid".to_string(),
                selectors: vec!["pushable".to_string(), "Wall".to_string()],
                source_line: "solid = pushable Wall".to_string(),
            },
            SelectorGroupDeclaration {
                name: "pushable".to_string(),
                selectors: vec!["Box".to_string(), "Crate".to_string()],
                source_line: "pushable = Box Crate".to_string(),
            },
        ];
        assert_eq!(
            expand_layer_selectors(&["solid"], &groups).unwrap(),
            ExpandedLayerSelectors {
                terms: vec!["Box".to_string(), "Crate".to_string(), "Wall".to_string()],
                used_groups: vec!["solid".to_string(), "pushable".to_string()],
            }
        );

        let cyclic = vec![
            SelectorGroupDeclaration {
                name: "a".to_string(),
                selectors: vec!["b".to_string()],
                source_line: "a = b".to_string(),
            },
            SelectorGroupDeclaration {
                name: "b".to_string(),
                selectors: vec!["a".to_string()],
                source_line: "b = a".to_string(),
            },
        ];
        assert_eq!(
            expand_layer_selectors(&["a"], &cyclic).unwrap_err().message,
            "group definitions cannot be cyclic"
        );
    }

    #[test]
    fn unresolved_pattern_owns_null_syntax() {
        parse_unresolved_pattern_syntax("[null | Player]").unwrap();
        assert_eq!(
            parse_unresolved_pattern_syntax("[no null]").unwrap_err(),
            UnresolvedPatternSyntaxError::CellPattern(CellPatternError::ForbidNull)
        );
        assert_eq!(
            parse_unresolved_pattern_syntax("[Player null]").unwrap_err(),
            UnresolvedPatternSyntaxError::CellPattern(CellPatternError::NullMixedWithOtherTokens)
        );
        assert_eq!(
            validate_null_pattern_cells([true]).unwrap_err(),
            NullCellPatternError::MissingAnchor
        );
        assert_eq!(
            validate_null_rewrite_cell(false, true, true).unwrap_err(),
            NullCellPatternError::IntroducedOnRewrite
        );
        assert_eq!(
            validate_null_rewrite_cell(true, false, false).unwrap_err(),
            NullCellPatternError::WriteToNull
        );
    }

    #[test]
    fn shared_rewrite_occurrence_diff_moves_subject_and_removes_absent_mark() {
        let before = vec![RewriteOccurrence {
            key: ("Player", 0),
            position: 0,
            subject: "Player",
            require_marks: vec!["right"],
            forbid_marks: Vec::new(),
        }];
        let after = vec![RewriteOccurrence {
            key: ("Player", 0),
            position: 1,
            subject: "Player",
            require_marks: Vec::new(),
            forbid_marks: Vec::new(),
        }];

        assert_eq!(
            diff_rewrite_occurrences(&before, &after, |left, right| left == right),
            vec![
                RewriteOccurrenceDelta::Move {
                    from: 0,
                    to: 1,
                    subject: "Player",
                },
                RewriteOccurrenceDelta::RemoveMark {
                    at: 1,
                    subject: "Player",
                    mark: "right",
                },
            ]
        );
    }
}
