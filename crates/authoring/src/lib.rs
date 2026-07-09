use std::ops::Range;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorCatalog<ObjectId, LayerId, Axis, Mark> {
    pub objects: Vec<ConcreteObject<ObjectId>>,
    pub families: Vec<ObjectFamily<ObjectId, Axis>>,
    pub groups: Vec<SelectorGroup<ObjectSelector<Mark>>>,
    object_layers: Vec<(ObjectId, LayerId)>,
}

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

pub fn is_display_object_token(token: &str) -> bool {
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
    let mut tokens = line.split_whitespace().collect::<Vec<_>>();
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
        let next = line[index + 1..].chars().next();
        if ch == '='
            && !matches!(previous, Some('=' | '!' | '<' | '>'))
            && !matches!(next, Some('='))
        {
            return Some((line[..index].trim(), line[index + 1..].trim()));
        }
    }
    None
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
    RuleStatements(Vec<String>),
    LifecycleCommands(Vec<String>),
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

pub fn collect_rule_program_block_body(
    lines: &[String],
    start: usize,
    block: RuleProgramBlockSurface<'_>,
) -> Result<(RuleProgramBlockBody, usize), RuleProgramBlockBodyError> {
    match block {
        RuleProgramBlockSurface::Rules { .. } => {
            collect_rule_statement_block_body(lines, start, "rules")
                .map(|(body, next)| (RuleProgramBlockBody::RuleStatements(body), next))
        }
        RuleProgramBlockSurface::OnLevelStart { .. } => {
            collect_rule_statement_block_body(lines, start, "on_level_start")
                .map(|(body, next)| (RuleProgramBlockBody::RuleStatements(body), next))
        }
        RuleProgramBlockSurface::OnLevelClear => {
            collect_lifecycle_command_block_body(lines, start, "on_level_clear")
                .map(|(body, next)| (RuleProgramBlockBody::LifecycleCommands(body), next))
        }
        RuleProgramBlockSurface::OnLastLevelClear => {
            collect_lifecycle_command_block_body(lines, start, "on_last_level_clear")
                .map(|(body, next)| (RuleProgramBlockBody::LifecycleCommands(body), next))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleStatementBlockSurface<'a> {
    Program(RuleProgramBlockSurface<'a>),
    Routine,
    DisplayHook,
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
        "on_display" => Some(RuleStatementBlockSurface::DisplayHook),
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
pub enum StandardRuleStepSurface {
    Move,
}

pub const STANDARD_RULE_STEP_NAMES: &[&str] = &["move"];

pub fn standard_rule_step_surface(line: &str) -> Option<StandardRuleStepSurface> {
    match line.trim() {
        "move" => Some(StandardRuleStepSurface::Move),
        _ => None,
    }
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
    if tokens.len() == 1
        && is_qualified_identifier(tokens[0])
        && standard_rule_step_surface(line).is_none()
    {
        return Ok(RuleStatementSurface::Call { name: tokens[0] });
    }
    rule_line_surface(line).map(RuleStatementSurface::RuleLine)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleLineSurface<'a> {
    StandardStep(StandardRuleStepSurface),
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
    StandardStep {
        step: StandardRuleStepSurface,
        span: Range<usize>,
    },
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
        RuleLineSurfaceSpans::StandardStep { step, .. } => RuleLineSurface::StandardStep(step),
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
    let line_text = &line[line_range.clone()];
    if let Some(step) = standard_rule_step_surface(line_text) {
        return Ok(RuleLineSurfaceSpans::StandardStep {
            step,
            span: line_range,
        });
    }
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
    let tokens = header_token_spans(line, rest.clone());
    let Some(orientation) = tokens.first() else {
        return Err(RuleLineSurfaceError::MissingOrientation);
    };
    let rewrite = trim_start_range(line, orientation.range.end..rest.end);
    if !line[rewrite.clone()].starts_with('[') {
        return Err(RuleLineSurfaceError::RewriteMustStartWithPattern);
    }
    Ok(RuleLineSurfaceSpans::OrientedRewrite {
        application,
        orientation: orientation.range.clone(),
        rewrite,
    })
}

pub fn rule_line_semantic_surface_spans(
    line: &str,
) -> Result<Vec<RuleSemanticSurfaceSpan>, RuleLineSurfaceError> {
    let mut spans = Vec::new();
    match rule_line_surface_spans(line)? {
        RuleLineSurfaceSpans::StandardStep { .. } => {}
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
        add_rule_mark_block_semantic_surface_spans(line, open + 1..token.end - 1, spans);
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

fn collect_rule_statement_block_body(
    lines: &[String],
    mut index: usize,
    block_name: &'static str,
) -> Result<(Vec<String>, usize), RuleProgramBlockBodyError> {
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
        let (rule_line, next_index) = collect_rule_statement_line(lines, index);
        body.push(rule_line);
        index = next_index;
    }
    Err(RuleProgramBlockBodyError::MissingClosingBrace { block_name })
}

fn collect_lifecycle_command_block_body(
    lines: &[String],
    mut index: usize,
    block_name: &'static str,
) -> Result<(Vec<String>, usize), RuleProgramBlockBodyError> {
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
        body.push(line.to_string());
        index += 1;
    }
    Err(RuleProgramBlockBodyError::MissingClosingBrace { block_name })
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
pub struct StandardMoveObject {
    pub object: u16,
    pub layer: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StandardMoveRulePlan {
    pub object: u16,
    pub direction_index: u16,
    pub layer_objects: Vec<u16>,
}

pub fn standard_move_rule_plans(
    objects: impl IntoIterator<Item = StandardMoveObject>,
    direction_count: u16,
) -> Vec<StandardMoveRulePlan> {
    let objects = objects.into_iter().collect::<Vec<_>>();
    let mut plans = Vec::new();
    for object in &objects {
        let layer_objects = objects
            .iter()
            .filter_map(|candidate| (candidate.layer == object.layer).then_some(candidate.object))
            .collect::<Vec<_>>();
        for direction_index in 0..direction_count {
            plans.push(StandardMoveRulePlan {
                object: object.object,
                direction_index,
                layer_objects: layer_objects.clone(),
            });
        }
    }
    plans
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellTokenError {
    UnmatchedCloseBrace,
    MissingCloseBrace,
    UnmatchedCloseParen,
    MissingCloseParen,
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
    fn new_puzzle_source_is_blank() {
        let source = new_puzzle_source("Custom Puzzle");

        assert_eq!(source, "");
        assert!(!source.contains("title "));
        assert!(!source.contains("puzzle main {"));
        assert!(!source.contains("levels "));
        assert!(!source.contains("keys {"));
        assert!(!source.contains("layers"));
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
    fn at_name_marks_display_object_tokens() {
        assert!(is_display_object_token("@Trail"));
        assert!(is_display_object_token("@Trail:kind"));
        assert!(is_display_object_token("@Trail{right}"));
        assert!(!is_display_object_token("Trail"));
        assert!(!is_display_object_token("@"));
        assert!(!is_display_object_token("@:kind"));
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
            standard_rule_step_surface("move"),
            Some(StandardRuleStepSurface::Move)
        );
        assert_eq!(
            rule_line_surface("move").unwrap(),
            RuleLineSurface::StandardStep(StandardRuleStepSurface::Move)
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
            "}".to_string(),
        ];
        assert_eq!(
            collect_rule_program_block_body(
                &rule_program_lines,
                0,
                RuleProgramBlockSurface::Rules { modifier: "" },
            )
            .unwrap(),
            (
                RuleProgramBlockBody::RuleStatements(vec![
                    "input directions [ Player | no Wall ] -> [ | Player ]".to_string(),
                    "move".to_string(),
                ]),
                6,
            )
        );
        let dense_multiline = vec![
            "right:up [ Player".to_string(),
            "Box ] -> [ Player".to_string(),
            "Box ]".to_string(),
        ];
        assert_eq!(
            collect_rule_statement_line(&dense_multiline, 0),
            (
                "right:up [ Player ; Box ] -> [ Player ; Box ]".to_string(),
                3,
            )
        );
        let lifecycle_lines = vec![
            "".to_string(),
            "if win_conditions -> next_level".to_string(),
            "}".to_string(),
        ];
        assert_eq!(
            collect_rule_program_block_body(
                &lifecycle_lines,
                0,
                RuleProgramBlockSurface::OnLevelClear,
            )
            .unwrap(),
            (
                RuleProgramBlockBody::LifecycleCommands(vec![
                    "if win_conditions -> next_level".to_string()
                ]),
                3,
            )
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
        assert!(projected.contains(&(RuleSemanticSurfaceKind::Mark, "mark")));
    }

    #[test]
    fn shared_cell_tokenizer_keeps_computed_axis_selectors_together() {
        assert_eq!(
            split_cell_tokens("Box:(facing + 90deg):red no Wall").unwrap(),
            vec!["Box:(facing + 90deg):red", "no", "Wall"]
        );
    }

    #[test]
    fn standard_move_plan_expands_objects_by_layer_and_direction() {
        let plans = standard_move_rule_plans(
            [
                StandardMoveObject {
                    object: 1,
                    layer: 0,
                },
                StandardMoveObject {
                    object: 2,
                    layer: 0,
                },
                StandardMoveObject {
                    object: 3,
                    layer: 1,
                },
            ],
            2,
        );

        assert_eq!(
            plans,
            vec![
                StandardMoveRulePlan {
                    object: 1,
                    direction_index: 0,
                    layer_objects: vec![1, 2],
                },
                StandardMoveRulePlan {
                    object: 1,
                    direction_index: 1,
                    layer_objects: vec![1, 2],
                },
                StandardMoveRulePlan {
                    object: 2,
                    direction_index: 0,
                    layer_objects: vec![1, 2],
                },
                StandardMoveRulePlan {
                    object: 2,
                    direction_index: 1,
                    layer_objects: vec![1, 2],
                },
                StandardMoveRulePlan {
                    object: 3,
                    direction_index: 0,
                    layer_objects: vec![3],
                },
                StandardMoveRulePlan {
                    object: 3,
                    direction_index: 1,
                    layer_objects: vec![3],
                },
            ]
        );
    }
}
