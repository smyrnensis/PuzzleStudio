use std::collections::BTreeMap;

use crate::{
    DenseCell3, DensePattern3, DenseRow3, DenseRuleTemplate3, DenseSlice3, Direction3,
    DirectionSet3, FrameOrientation3, FrameSlot3, Game3, InputDef3, InputId3, LayerId, Level3,
    LevelBundle3, LevelCell3, LevelEntry3, Lifecycle3, LifecycleCommand3, LineMatchCellTemplate3,
    LineOrientation3, LinePatternTemplate3, LineRuleTemplate3, LineWriteOpTemplate3,
    LocalWriteOpTemplate3, MatchCell3, ObjectDef3, ObjectFamily3, ObjectId, ObjectSelector3,
    ObjectVariant3, Offset3, Pattern3, Rule3, RuleEffect3, SelectorCatalog3, SelectorGroup3,
    SelectorTag3, Size3, Sprite3, SpriteColor3, SpriteSet3, SpriteVoxels3, VariantAxis3,
    WinCondition3, lower_dense_rule_template, lower_line_rule_template,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedPuzzle3 {
    pub game: Game3,
    pub catalog: SelectorCatalog3,
    pub settings: ModelSettings3,
    pub rules: Vec<Rule3>,
    pub level_bundle: Option<LevelBundle3>,
    pub win_condition: Option<WinCondition3>,
    pub lifecycle: Lifecycle3,
    pub sprite_set: Option<SpriteSet3>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelSettings3 {
    pub camera: CameraSettings3,
    pub grid: GridSettings3,
    pub sprite: SpriteRenderSettings3,
}

impl Default for ModelSettings3 {
    fn default() -> Self {
        Self {
            camera: CameraSettings3::default(),
            grid: GridSettings3::default(),
            sprite: SpriteRenderSettings3::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraSettings3 {
    pub yaw_degrees: i16,
    pub pitch_degrees: i16,
    pub zoom_milli: u16,
    pub interactive_look: bool,
    pub interactive_zoom: bool,
}

impl Default for CameraSettings3 {
    fn default() -> Self {
        Self {
            yaw_degrees: 34,
            pitch_degrees: 38,
            zoom_milli: 1100,
            interactive_look: false,
            interactive_zoom: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GridSettings3 {
    pub occupied_cells: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteRenderSettings3 {
    pub shade: bool,
}

impl Default for SpriteRenderSettings3 {
    fn default() -> Self {
        Self { shade: true }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError3 {
    Message(String),
}

impl From<puzzle_scene::SceneBlockParseError> for ParseError3 {
    fn from(value: puzzle_scene::SceneBlockParseError) -> Self {
        Self::Message(value.to_string())
    }
}

pub fn parse_puzzle3d(source: &str) -> Result<ParsedPuzzle3, ParseError3> {
    Parser3::new(source).parse()
}

struct Parser3 {
    lines: Vec<String>,
    value_sets: Vec<(String, Vec<String>)>,
    input_specs: Vec<InputSpec3>,
    layers: Vec<String>,
    object_specs: Vec<ObjectSpec3>,
    group_specs: Vec<GroupSpec3>,
    legend_specs: Vec<LegendSpec3>,
    level_specs: Vec<LevelSpec3>,
    rule_lines: Vec<String>,
    on_level_start_lines: Vec<String>,
    on_level_clear_lines: Vec<String>,
    win_condition_lines: Vec<String>,
    settings: ModelSettings3,
    sprite_set: Option<SpriteSet3>,
}

impl Parser3 {
    fn new(source: &str) -> Self {
        Self {
            lines: source
                .lines()
                .map(strip_comment)
                .map(str::trim)
                .map(str::to_string)
                .collect(),
            value_sets: Vec::new(),
            input_specs: Vec::new(),
            layers: Vec::new(),
            object_specs: Vec::new(),
            group_specs: Vec::new(),
            legend_specs: Vec::new(),
            level_specs: Vec::new(),
            rule_lines: Vec::new(),
            on_level_start_lines: Vec::new(),
            on_level_clear_lines: Vec::new(),
            win_condition_lines: Vec::new(),
            settings: ModelSettings3::default(),
            sprite_set: None,
        }
    }

    fn parse(mut self) -> Result<ParsedPuzzle3, ParseError3> {
        let mut index = 0;
        while index < self.lines.len() {
            let line = self.lines[index].clone();
            if line.is_empty() {
                index += 1;
            } else if is_model3_header(&line) {
                index = self.parse_model_block(index + 1)?;
            } else if line.starts_with("model puzzle3 ") {
                return Err(message(
                    "top-level 3D puzzle definition must be: puzzle3 <name>",
                ));
            } else if is_document_metadata_line(&line) {
                index += 1;
            } else if is_document_shell_block(&line) {
                index = skip_braced_block(&self.lines, index + 1)?;
            } else if line == "layers {" {
                index = self.parse_layers_block(index + 1)?;
            } else if line == "objects {" {
                index = self.parse_objects_block(index + 1)?;
            } else if line == "inputs {" {
                index = self.parse_inputs_block(index + 1)?;
            } else if line == "groups {" || line == "group {" {
                index = self.parse_groups_block(index + 1)?;
            } else if line == "legend {" {
                index = self.parse_legend_block(index + 1)?;
            } else if is_levels3_header(&line) {
                index = self.parse_levels_block(index + 1)?;
            } else if is_sprites3_header(&line) {
                index = self.parse_sprites3_block(index + 1, &line)?;
            } else if let Some(name) = parse_scene_header(&line) {
                let _ = name;
                index = skip_braced_block(&self.lines, index + 1)?;
            } else if line == "rules {" {
                index = self.parse_rules_block(index + 1)?;
            } else if line == "on_level_start {" {
                index = self.parse_on_level_start_block(index + 1)?;
            } else if line == "on_level_clear {" {
                index = self.parse_on_level_clear_block(index + 1)?;
            } else if line == "win_conditions {" {
                index = self.parse_win_conditions_block(index + 1)?;
            } else if line == "render {" {
                index = self.parse_render_block(index + 1)?;
            } else if let Some(setting) = parse_model_setting_line(&line)? {
                self.apply_model_setting(setting);
                index += 1;
            } else if let Some(rest) = line.strip_prefix("group ") {
                self.group_specs.push(parse_group_spec(rest)?);
                index += 1;
            } else if line.contains('=') {
                self.value_sets.push(parse_value_set(&line)?);
                index += 1;
            } else {
                return Err(message(format!("unknown 3D puzzle directive: {line}")));
            }
        }

        let layer_count = self.layers.len() as u16;
        if layer_count == 0 {
            return Err(message("missing layers block"));
        }

        let mut build = CatalogBuild3::new(self.value_sets, self.layers);
        for spec in self.object_specs {
            build.add_object_spec(spec)?;
        }
        let object_defs = build.object_defs.clone();
        let catalog = build.catalog_with_groups(self.group_specs)?;
        let game = Game3::new_with_inputs(
            layer_count,
            object_defs,
            inputs_from_specs(self.input_specs)?,
        );

        let mut rules = Vec::new();
        for line in &self.rule_lines {
            rules.extend(parse_rule_line(line, &catalog)?);
        }
        let mut on_level_start = Vec::new();
        for line in &self.on_level_start_lines {
            on_level_start.extend(parse_rule_line(line, &catalog)?);
        }
        let on_level_clear = self
            .on_level_clear_lines
            .iter()
            .map(|line| parse_lifecycle_command_line(line))
            .collect::<Result<Vec<_>, ParseError3>>()?;
        let lifecycle = Lifecycle3::new(on_level_start, on_level_clear);
        let win_condition = if self.win_condition_lines.is_empty() {
            None
        } else {
            Some(lower_win_conditions(&catalog, &self.win_condition_lines)?)
        };
        let level_bundle = if self.level_specs.is_empty() {
            None
        } else {
            Some(lower_level_bundle(
                game.clone(),
                &catalog,
                &self.legend_specs,
                &self.level_specs,
            )?)
        };

        Ok(ParsedPuzzle3 {
            game,
            catalog,
            settings: self.settings,
            rules,
            level_bundle,
            win_condition,
            lifecycle,
            sprite_set: self.sprite_set,
        })
    }

    fn parse_model_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = self.lines[index].clone();
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
            } else if line == "layers {" {
                index = self.parse_layers_block(index + 1)?;
            } else if line == "objects {" {
                index = self.parse_objects_block(index + 1)?;
            } else if line == "inputs {" {
                index = self.parse_inputs_block(index + 1)?;
            } else if line == "groups {" || line == "group {" {
                index = self.parse_groups_block(index + 1)?;
            } else if line == "rules {" {
                index = self.parse_rules_block(index + 1)?;
            } else if line == "on_level_start {" {
                index = self.parse_on_level_start_block(index + 1)?;
            } else if line == "on_level_clear {" {
                index = self.parse_on_level_clear_block(index + 1)?;
            } else if line == "win_conditions {" {
                index = self.parse_win_conditions_block(index + 1)?;
            } else if line == "render {" {
                index = self.parse_render_block(index + 1)?;
            } else if let Some(setting) = parse_model_setting_line(&line)? {
                self.apply_model_setting(setting);
                index += 1;
            } else if is_sprites3_header(&line) {
                index = self.parse_sprites3_block(index + 1, &line)?;
            } else if let Some(name) = parse_scene_header(&line) {
                let _ = name;
                index = skip_braced_block(&self.lines, index + 1)?;
            } else if let Some(rest) = line.strip_prefix("group ") {
                self.group_specs.push(parse_group_spec(rest)?);
                index += 1;
            } else if line.contains('=') {
                self.value_sets.push(parse_value_set(&line)?);
                index += 1;
            } else {
                return Err(message(format!("unknown model directive: {line}")));
            }
        }
        Err(message("model block missing }"))
    }

    fn apply_model_setting(&mut self, setting: ModelSetting3) {
        match setting {
            ModelSetting3::LegacyDebugCamera(value) => {
                self.settings.camera.interactive_look = value;
            }
            ModelSetting3::CameraYaw(value) => self.settings.camera.yaw_degrees = value,
            ModelSetting3::CameraPitch(value) => self.settings.camera.pitch_degrees = value,
            ModelSetting3::CameraZoom(value) => self.settings.camera.zoom_milli = value,
            ModelSetting3::InteractiveLook(value) => self.settings.camera.interactive_look = value,
            ModelSetting3::InteractiveZoom(value) => self.settings.camera.interactive_zoom = value,
            ModelSetting3::OccupiedCellGrid(value) => self.settings.grid.occupied_cells = value,
            ModelSetting3::SpriteShade(value) => self.settings.sprite.shade = value,
        }
    }

    fn parse_render_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = self.lines[index].clone();
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
            } else if line == "camera {" {
                index = self.parse_camera_block(index + 1)?;
            } else if line == "grid {" {
                index = self.parse_grid_block(index + 1)?;
            } else {
                let setting = parse_render_setting_line(&line)?;
                self.apply_model_setting(setting);
                index += 1;
            }
        }
        Err(message("render block missing }"))
    }

    fn parse_camera_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = self.lines[index].clone();
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            let setting = parse_camera_setting_line(&line)?;
            self.apply_model_setting(setting);
            index += 1;
        }
        Err(message("camera block missing }"))
    }

    fn parse_grid_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = self.lines[index].clone();
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            let setting = parse_grid_setting_line(&line)?;
            self.apply_model_setting(setting);
            index += 1;
        }
        Err(message("grid block missing }"))
    }

    fn parse_sprites3_block(
        &mut self,
        mut index: usize,
        header: &str,
    ) -> Result<usize, ParseError3> {
        if self.sprite_set.is_some() {
            return Err(message("duplicate sprites3 block"));
        }
        let (name, model) = parse_sprites3_header(header)?;
        let mut sprites = Vec::new();
        while index < self.lines.len() {
            let line = self.lines[index].clone();
            if line == "}" {
                self.sprite_set = Some(SpriteSet3::new(name, model, sprites));
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            if line.starts_with("sprite ") {
                return Err(message(
                    "sprites3 entries must use canonical form: <name>, palette row, voxel rows",
                ));
            }
            if is_canonical_sprite_name(&line) {
                let sprite_name = line.clone();
                if sprites.iter().any(|sprite| sprite.name == sprite_name) {
                    return Err(message(format!("duplicate sprite: {sprite_name}")));
                }
                let (next, sprite) = self.parse_canonical_sprite(index + 1, sprite_name)?;
                sprites.push(sprite);
                index = next;
                continue;
            }
            return Err(message(format!("invalid sprites3 sprite name: {line}")));
        }
        Err(message("sprites3 block missing }"))
    }

    fn parse_canonical_sprite(
        &self,
        mut index: usize,
        name: String,
    ) -> Result<(usize, Sprite3), ParseError3> {
        while index < self.lines.len() && self.lines[index].is_empty() {
            index += 1;
        }
        if index >= self.lines.len() || self.lines[index] == "}" {
            return Err(message(format!("sprite {name} missing palette row")));
        }
        let palette = parse_canonical_sprite_palette_line(&self.lines[index])?;
        index += 1;

        while index < self.lines.len() && self.lines[index].is_empty() {
            index += 1;
        }

        let mut rows = Vec::new();
        while index < self.lines.len() {
            let line = self.lines[index].clone();
            if line == "}" {
                break;
            }
            if line.starts_with("sprite ") || line == "colors {" || line == "voxels {" {
                return Err(message(
                    "sprites3 entries must use canonical form: <name>, palette row, voxel rows",
                ));
            }
            if !rows.is_empty() && self.is_canonical_sprite_start(index) {
                break;
            }
            rows.push(line);
            index += 1;
        }
        let voxels = parse_sprite_voxels(&name, &rows, &palette)?;
        Ok((index, Sprite3::new(name, palette, voxels)))
    }

    fn is_canonical_sprite_start(&self, index: usize) -> bool {
        if index >= self.lines.len() || !is_canonical_sprite_name(&self.lines[index]) {
            return false;
        }
        let mut next = index + 1;
        while next < self.lines.len() && self.lines[next].is_empty() {
            next += 1;
        }
        next < self.lines.len() && is_canonical_sprite_palette_line(&self.lines[next])
    }

    fn parse_layers_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = self.lines[index].clone();
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            self.parse_layer_line(&line)?;
            index += 1;
        }
        Err(message("layers block missing }"))
    }

    fn parse_layer_line(&mut self, line: &str) -> Result<(), ParseError3> {
        if let Some((layer, objects)) = line.split_once('=') {
            let layer = layer.trim();
            if layer.is_empty() {
                return Err(message("layer declaration must name a layer before ="));
            }
            self.layers.push(layer.to_string());
            for object in objects.split_whitespace() {
                self.object_specs
                    .push(parse_layer_object_spec(object, layer)?);
            }
            return Ok(());
        }
        self.layers
            .extend(line.split_whitespace().map(str::to_string));
        Ok(())
    }

    fn parse_objects_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = &self.lines[index];
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            self.object_specs.push(parse_object_spec(line)?);
            index += 1;
        }
        Err(message("objects block missing }"))
    }

    fn parse_inputs_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = &self.lines[index];
            if line == "}" {
                return Ok(index + 1);
            }
            if !line.is_empty() {
                self.input_specs.push(parse_input_spec(line)?);
            }
            index += 1;
        }
        Err(message("inputs block missing }"))
    }

    fn parse_groups_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = &self.lines[index];
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            self.group_specs.push(parse_group_spec(line)?);
            index += 1;
        }
        Err(message("groups block missing }"))
    }

    fn parse_legend_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = &self.lines[index];
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            self.legend_specs.push(parse_legend_spec(line)?);
            index += 1;
        }
        Err(message("legend block missing }"))
    }

    fn parse_levels_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = self.lines[index].clone();
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            if line == "legend {" {
                index = self.parse_legend_block(index + 1)?;
                continue;
            }
            if let Some(name) = parse_level_header(&line) {
                let (next, rows) = self.collect_level_body(index + 1)?;
                self.level_specs.push(LevelSpec3 { name, rows });
                index = next;
                continue;
            }
            return Err(message(format!("unknown levels directive: {line}")));
        }
        Err(message("levels block missing }"))
    }

    fn collect_level_body(&self, mut index: usize) -> Result<(usize, Vec<String>), ParseError3> {
        let mut rows = Vec::new();
        while index < self.lines.len() {
            let line = &self.lines[index];
            if line == "}" {
                return Ok((index + 1, rows));
            }
            rows.push(line.clone());
            index += 1;
        }
        Err(message("level block missing }"))
    }

    fn parse_rules_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = &self.lines[index];
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            self.rule_lines.push(line.clone());
            index += 1;
        }
        Err(message("rules block missing }"))
    }

    fn parse_on_level_start_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = &self.lines[index];
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            self.on_level_start_lines.push(line.clone());
            index += 1;
        }
        Err(message("on_level_start block missing }"))
    }

    fn parse_on_level_clear_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = &self.lines[index];
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            self.on_level_clear_lines.push(line.clone());
            index += 1;
        }
        Err(message("on_level_clear block missing }"))
    }

    fn parse_win_conditions_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = &self.lines[index];
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            self.win_condition_lines.push(line.clone());
            index += 1;
        }
        Err(message("win_conditions block missing }"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ObjectSpec3 {
    name: String,
    axes: Vec<String>,
    layer: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct InputSpec3 {
    name: String,
    keys: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GroupSpec3 {
    name: String,
    selectors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LegendSpec3 {
    ch: char,
    selectors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LevelSpec3 {
    name: String,
    rows: Vec<String>,
}

struct CatalogBuild3 {
    value_sets: Vec<(String, Vec<String>)>,
    layers: Vec<String>,
    next_object: u16,
    concrete: Vec<crate::ConcreteObject3>,
    families: Vec<ObjectFamily3>,
    object_defs: Vec<ObjectDef3>,
}

impl CatalogBuild3 {
    fn new(value_sets: Vec<(String, Vec<String>)>, layers: Vec<String>) -> Self {
        Self {
            value_sets,
            layers,
            next_object: 1,
            concrete: Vec::new(),
            families: Vec::new(),
            object_defs: Vec::new(),
        }
    }

    fn add_object_spec(&mut self, spec: ObjectSpec3) -> Result<(), ParseError3> {
        let layer = self.layer_id(&spec.layer)?;
        if spec.axes.is_empty() {
            let id = self.alloc_object();
            self.concrete
                .push(crate::ConcreteObject3::new(id, spec.name));
            self.object_defs.push(ObjectDef3 {
                id,
                layer_id: layer,
            });
            return Ok(());
        }

        let mut axes = Vec::new();
        let mut axis_values = Vec::new();
        for axis in &spec.axes {
            let (axis_def, values) = self.axis_def(axis)?;
            axes.push(axis_def);
            axis_values.push(values);
        }

        let mut variants = Vec::new();
        for values in cartesian_values(&axis_values) {
            let id = self.alloc_object();
            variants.push(ObjectVariant3::new(id, values.clone()));
            self.object_defs.push(ObjectDef3 {
                id,
                layer_id: layer,
            });
        }
        self.families
            .push(ObjectFamily3::new(spec.name, axes, variants));
        Ok(())
    }

    fn catalog_with_groups(
        self,
        group_specs: Vec<GroupSpec3>,
    ) -> Result<SelectorCatalog3, ParseError3> {
        let mut groups = Vec::new();
        for spec in group_specs {
            let selectors = spec
                .selectors
                .iter()
                .map(|selector| parse_selector(selector, &self.families, &groups))
                .collect::<Result<Vec<_>, _>>()?;
            groups.push(SelectorGroup3::new(spec.name, selectors));
        }
        SelectorCatalog3::checked_new(self.concrete, self.families, groups)
            .map_err(|error| message(format!("invalid selector catalog: {error:?}")))
    }

    fn alloc_object(&mut self) -> ObjectId {
        let id = ObjectId(self.next_object);
        self.next_object += 1;
        id
    }

    fn layer_id(&self, name: &str) -> Result<LayerId, ParseError3> {
        self.layers
            .iter()
            .position(|layer| layer == name)
            .map(|index| LayerId(index as u16))
            .ok_or_else(|| message(format!("unknown layer: {name}")))
    }

    fn axis_def(&self, axis: &str) -> Result<(VariantAxis3, Vec<String>), ParseError3> {
        match axis {
            "directions" => Ok((
                VariantAxis3::directions(axis, DirectionSet3::Directions),
                direction_names(DirectionSet3::Directions),
            )),
            "horizontal" => Ok((
                VariantAxis3::directions(axis, DirectionSet3::Horizontal),
                direction_names(DirectionSet3::Horizontal),
            )),
            "vertical" => Ok((
                VariantAxis3::directions(axis, DirectionSet3::Vertical),
                direction_names(DirectionSet3::Vertical),
            )),
            _ => {
                let values = self
                    .value_sets
                    .iter()
                    .find_map(|(name, values)| (name == axis).then_some(values.clone()))
                    .ok_or_else(|| message(format!("unknown object axis: {axis}")))?;
                Ok((VariantAxis3::named(axis, values.clone()), values))
            }
        }
    }
}

fn parse_value_set(line: &str) -> Result<(String, Vec<String>), ParseError3> {
    let (name, values) = line
        .split_once('=')
        .ok_or_else(|| message("value set must be: name = value..."))?;
    let values = values
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(message("value set must contain at least one value"));
    }
    Ok((name.trim().to_string(), values))
}

enum ModelSetting3 {
    LegacyDebugCamera(bool),
    CameraYaw(i16),
    CameraPitch(i16),
    CameraZoom(u16),
    InteractiveLook(bool),
    InteractiveZoom(bool),
    OccupiedCellGrid(bool),
    SpriteShade(bool),
}

fn parse_model_setting_line(line: &str) -> Result<Option<ModelSetting3>, ParseError3> {
    let Some((name, value)) = line.split_once('=') else {
        return Ok(None);
    };
    let name = name.trim();
    let value = value.trim();
    match name {
        "debug_camera" => Ok(Some(ModelSetting3::LegacyDebugCamera(parse_bool_setting(
            value, name,
        )?))),
        "camera_yaw" => Ok(Some(ModelSetting3::CameraYaw(parse_degrees_setting(
            value, name,
        )?))),
        "camera_pitch" => Ok(Some(ModelSetting3::CameraPitch(parse_degrees_setting(
            value, name,
        )?))),
        "camera_zoom" => Ok(Some(ModelSetting3::CameraZoom(parse_zoom_milli_setting(
            value, name,
        )?))),
        _ => Ok(None),
    }
}

fn parse_camera_setting_line(line: &str) -> Result<ModelSetting3, ParseError3> {
    let (name, value) = parse_setting_assignment(line, "camera setting")?;
    match name {
        "yaw" => Ok(ModelSetting3::CameraYaw(parse_degrees_setting(
            value, name,
        )?)),
        "pitch" => Ok(ModelSetting3::CameraPitch(parse_degrees_setting(
            value, name,
        )?)),
        "zoom" => Ok(ModelSetting3::CameraZoom(parse_zoom_milli_setting(
            value, name,
        )?)),
        "interactive_look" => Ok(ModelSetting3::InteractiveLook(parse_bool_setting(
            value, name,
        )?)),
        "interactive_zoom" => Ok(ModelSetting3::InteractiveZoom(parse_bool_setting(
            value, name,
        )?)),
        _ => Err(message(format!("unknown camera setting: {name}"))),
    }
}

fn parse_grid_setting_line(line: &str) -> Result<ModelSetting3, ParseError3> {
    let (name, value) = parse_setting_assignment(line, "grid setting")?;
    match name {
        "occupied_cells" => Ok(ModelSetting3::OccupiedCellGrid(parse_bool_setting(
            value, name,
        )?)),
        _ => Err(message(format!("unknown grid setting: {name}"))),
    }
}

fn parse_render_setting_line(line: &str) -> Result<ModelSetting3, ParseError3> {
    let (name, value) = parse_setting_assignment(line, "render setting")?;
    match name {
        "shade" => Ok(ModelSetting3::SpriteShade(parse_bool_setting(value, name)?)),
        _ => Err(message(format!("unknown render setting: {name}"))),
    }
}

fn parse_setting_assignment<'a>(
    line: &'a str,
    context: &str,
) -> Result<(&'a str, &'a str), ParseError3> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    match tokens.as_slice() {
        [name, value] => Ok((*name, *value)),
        [name, "=", value] => Ok((*name, *value)),
        _ => Err(message(format!("{context} must be: <name> = <value>"))),
    }
}

fn parse_bool_setting(value: &str, name: &str) -> Result<bool, ParseError3> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(message(format!("{name} must be true or false"))),
    }
}

fn parse_degrees_setting(value: &str, name: &str) -> Result<i16, ParseError3> {
    value
        .parse::<i16>()
        .map_err(|_| message(format!("{name} must be an integer degree value")))
}

fn parse_zoom_milli_setting(value: &str, name: &str) -> Result<u16, ParseError3> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || !whole.chars().all(|ch| ch.is_ascii_digit())
        || !fraction.chars().all(|ch| ch.is_ascii_digit())
        || fraction.len() > 3
    {
        return Err(message(format!(
            "{name} must be a positive number with at most three decimal places",
        )));
    }
    let whole = whole
        .parse::<u32>()
        .map_err(|_| message(format!("{name} must be a positive number")))?;
    let fraction = format!("{fraction:0<3}")
        .parse::<u32>()
        .map_err(|_| message(format!("{name} must be a positive number")))?;
    let milli = whole
        .checked_mul(1000)
        .and_then(|value| value.checked_add(fraction))
        .ok_or_else(|| message(format!("{name} is too large")))?;
    if milli == 0 || milli > u32::from(u16::MAX) {
        return Err(message(format!(
            "{name} must be greater than 0 and not too large"
        )));
    }
    Ok(milli as u16)
}

fn parse_object_spec(line: &str) -> Result<ObjectSpec3, ParseError3> {
    let mut parts = line.split_whitespace();
    let name = parts
        .next()
        .ok_or_else(|| message("object row must be: Object[:axis...] layer"))?;
    let layer = parts
        .next()
        .ok_or_else(|| message("object row must be: Object[:axis...] layer"))?;
    if parts.next().is_some() {
        return Err(message("object row must be: Object[:axis...] layer"));
    }
    let mut name_parts = name.split(':');
    let base = name_parts.next().unwrap().to_string();
    Ok(ObjectSpec3 {
        name: base,
        axes: name_parts.map(str::to_string).collect(),
        layer: layer.to_string(),
    })
}

fn parse_input_spec(line: &str) -> Result<InputSpec3, ParseError3> {
    let (name, keys) = line
        .split_once("<-")
        .ok_or_else(|| message("inputs row must be: <input> <- <key...>"))?;
    let name = name.trim();
    if name.is_empty() {
        return Err(message("inputs row must name an input before <-"));
    }
    let keys = keys
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if keys.is_empty() {
        return Err(message("inputs row must include at least one key after <-"));
    }
    Ok(InputSpec3 {
        name: name.to_string(),
        keys,
    })
}

fn parse_layer_object_spec(token: &str, layer: &str) -> Result<ObjectSpec3, ParseError3> {
    if token == "empty" {
        return Err(message("empty cannot be generated as a layer object"));
    }
    let mut name_parts = token.split(':');
    let base = name_parts
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| message("layer object must be Object[:axis...]"))?
        .to_string();
    Ok(ObjectSpec3 {
        name: base,
        axes: name_parts.map(str::to_string).collect(),
        layer: layer.to_string(),
    })
}

fn parse_group_spec(line: &str) -> Result<GroupSpec3, ParseError3> {
    let (name, selectors) = line
        .split_once('=')
        .ok_or_else(|| message("group row must be: name = selector..."))?;
    let selectors = selectors
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if selectors.is_empty() {
        return Err(message("group must contain at least one selector"));
    }
    Ok(GroupSpec3 {
        name: name.trim().to_string(),
        selectors,
    })
}

fn parse_legend_spec(line: &str) -> Result<LegendSpec3, ParseError3> {
    let (ch, selectors) = line
        .split_once('=')
        .ok_or_else(|| message("legend row must be: <char> = selector..."))?;
    let ch = parse_legend_char(ch.trim())?;
    let selectors = selectors
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if selectors.is_empty() {
        return Err(message("legend row must contain at least one selector"));
    }
    if selectors.iter().any(|selector| selector == "empty") && selectors.len() > 1 {
        return Err(message("empty cannot be mixed with object selectors"));
    }
    Ok(LegendSpec3 { ch, selectors })
}

fn parse_legend_char(token: &str) -> Result<char, ParseError3> {
    let mut chars = token.chars();
    let Some(ch) = chars.next() else {
        return Err(message("legend row must start with a character"));
    };
    if chars.next().is_some() {
        return Err(message(format!(
            "legend key must be one character: {token}"
        )));
    }
    Ok(ch)
}

fn parse_level_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("level ")?;
    let name = rest.strip_suffix('{')?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn parse_scene_header(line: &str) -> Option<String> {
    let rest = line.strip_prefix("scene ")?;
    let name = rest.strip_suffix('{')?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn is_model3_header(line: &str) -> bool {
    line.strip_prefix("puzzle3 ")
        .is_some_and(|rest| rest.ends_with('{'))
}

fn is_document_metadata_line(line: &str) -> bool {
    line.starts_with("title ")
        || line.starts_with("subtitle ")
        || line.starts_with("author ")
        || line.starts_with("homepage ")
        || line.starts_with("default_wait_time ")
}

fn is_document_shell_block(line: &str) -> bool {
    matches!(line, "sounds {" | "theme {" | "assets {")
        || line
            .strip_prefix("theme ")
            .is_some_and(|rest| rest.ends_with('{'))
}

fn skip_braced_block(lines: &[String], mut index: usize) -> Result<usize, ParseError3> {
    let mut depth = 1i32;
    while index < lines.len() {
        let line = &lines[index];
        depth += line.chars().filter(|ch| *ch == '{').count() as i32;
        depth -= line.chars().filter(|ch| *ch == '}').count() as i32;
        index += 1;
        if depth == 0 {
            return Ok(index);
        }
    }
    Err(message("document shell block missing }"))
}

fn is_levels3_header(line: &str) -> bool {
    line == "levels3 {"
        || line
            .strip_prefix("levels3 ")
            .is_some_and(|rest| rest.ends_with('{'))
}

fn is_sprites3_header(line: &str) -> bool {
    line == "sprites3 {"
        || line
            .strip_prefix("sprites3 ")
            .is_some_and(|rest| rest.ends_with('{'))
}

fn parse_sprites3_header(line: &str) -> Result<(String, Option<String>), ParseError3> {
    let header = line
        .strip_prefix("sprites3")
        .and_then(|rest| rest.strip_suffix('{'))
        .ok_or_else(|| message("sprites3 block must end with {"))?
        .trim();
    if header.is_empty() {
        return Ok(("default".to_string(), None));
    }
    let parts = header.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [name] => Ok(((*name).to_string(), None)),
        [name, "of", model] => Ok(((*name).to_string(), Some((*model).to_string()))),
        _ => Err(message(
            "sprites3 header must be: sprites3 [name [of model]] {",
        )),
    }
}

fn lower_level_bundle(
    game: Game3,
    catalog: &SelectorCatalog3,
    legend_specs: &[LegendSpec3],
    level_specs: &[LevelSpec3],
) -> Result<LevelBundle3, ParseError3> {
    let legend = lower_legend(catalog, legend_specs)?;
    let levels = level_specs
        .iter()
        .map(|spec| {
            Ok(LevelEntry3::new(
                spec.name.clone(),
                lower_level_spec(&legend, spec)?,
            ))
        })
        .collect::<Result<Vec<_>, ParseError3>>()?;
    LevelBundle3::checked_new(game, levels)
        .map_err(|error| message(format!("invalid level bundle: {error:?}")))
}

fn lower_legend(
    catalog: &SelectorCatalog3,
    specs: &[LegendSpec3],
) -> Result<BTreeMap<char, Vec<ObjectId>>, ParseError3> {
    let mut legend = BTreeMap::new();
    let mut has_empty_override = false;
    for spec in specs {
        if legend.contains_key(&spec.ch) {
            return Err(message(format!("duplicate legend char: {}", spec.ch)));
        }
        let mut objects = Vec::new();
        if spec.selectors.len() == 1 && spec.selectors[0] == "empty" {
            has_empty_override = true;
        } else {
            for token in &spec.selectors {
                let selector = parse_selector(token, &catalog.families, &catalog.groups)?;
                let resolved = catalog
                    .resolve(&selector)
                    .map_err(|error| message(format!("invalid legend selector: {error:?}")))?;
                for object in resolved.alternatives {
                    push_unique_object(&mut objects, object);
                }
            }
        }
        legend.insert(spec.ch, objects);
    }
    if !has_empty_override && !legend.contains_key(&'.') {
        legend.insert('.', Vec::new());
    }
    Ok(legend)
}

fn lower_level_spec(
    legend: &BTreeMap<char, Vec<ObjectId>>,
    spec: &LevelSpec3,
) -> Result<Level3, ParseError3> {
    let slices = split_level_slices(&spec.rows)?;
    let depth = slices[0].len();
    let width = slices[0][0].chars().count();
    if width == 0 {
        return Err(message(format!("level {} has an empty row", spec.name)));
    }

    let size = Size3::new(width as u16, depth as u16, slices.len() as u16);
    let mut cells = Vec::new();
    for (z, slice) in slices.iter().enumerate() {
        if slice.len() != depth {
            return Err(message(format!(
                "level {} slices must have the same depth",
                spec.name
            )));
        }
        for (y, row) in slice.iter().enumerate() {
            if row.chars().count() != width {
                return Err(message(format!(
                    "level {} slices must have the same width",
                    spec.name
                )));
            }
            for (x, ch) in row.chars().enumerate() {
                let objects = legend.get(&ch).ok_or_else(|| {
                    message(format!(
                        "level {} uses unknown legend char: {ch}",
                        spec.name
                    ))
                })?;
                if objects.is_empty() {
                    continue;
                }
                cells.push(LevelCell3::new(
                    crate::Coord3::from_standard_text_position(size, x as u16, y as u16, z as u16),
                    objects.clone(),
                ));
            }
        }
    }

    Ok(Level3::new(size, cells))
}

fn is_canonical_sprite_name(line: &str) -> bool {
    !line.is_empty()
        && !line.contains(char::is_whitespace)
        && !line.contains('{')
        && !line.contains('}')
        && !line.contains('=')
        && !is_canonical_sprite_palette_line(line)
        && line
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':' | '@'))
}

fn is_canonical_sprite_palette_line(line: &str) -> bool {
    let mut tokens = line.split_whitespace().peekable();
    tokens.peek().is_some() && tokens.all(|token| token == "transparent" || is_hex_color(token))
}

fn parse_canonical_sprite_palette_line(
    line: &str,
) -> Result<BTreeMap<char, SpriteColor3>, ParseError3> {
    if !is_canonical_sprite_palette_line(line) {
        return Err(message(
            "sprite palette row must be whitespace-separated <color|transparent> values",
        ));
    }
    let mut palette = BTreeMap::new();
    for (index, token) in line.split_whitespace().enumerate() {
        let key = sprite_palette_key(index)?;
        let color = if token == "transparent" {
            SpriteColor3::Transparent
        } else {
            SpriteColor3::Hex(token.to_string())
        };
        palette.insert(key, color);
    }
    Ok(palette)
}

fn sprite_palette_key(index: usize) -> Result<char, ParseError3> {
    const KEYS: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    KEYS.chars()
        .nth(index)
        .ok_or_else(|| message("sprite palette supports at most 62 colors"))
}

fn is_hex_color(value: &str) -> bool {
    value.starts_with('#')
        && matches!(value.len(), 4 | 5 | 7 | 9)
        && value[1..].chars().all(|ch| ch.is_ascii_hexdigit())
}

fn parse_sprite_voxels(
    sprite_name: &str,
    rows: &[String],
    palette: &BTreeMap<char, SpriteColor3>,
) -> Result<SpriteVoxels3, ParseError3> {
    let slices = split_level_slices(rows)?;
    let depth = slices[0].len();
    let width = slices[0][0].chars().count();
    if width == 0 {
        return Err(message(format!("sprite {sprite_name} has an empty row")));
    }
    for slice in &slices {
        if slice.len() != depth {
            return Err(message(format!(
                "sprite {sprite_name} slices must have the same depth",
            )));
        }
        for row in slice {
            if row.chars().count() != width {
                return Err(message(format!(
                    "sprite {sprite_name} slices must have the same width",
                )));
            }
            for ch in row.chars() {
                if is_implicit_transparent_sprite_char(ch) {
                    continue;
                }
                if !palette.contains_key(&ch) {
                    return Err(message(format!(
                        "sprite {sprite_name} uses undefined color key: {ch}",
                    )));
                }
            }
        }
    }
    Ok(SpriteVoxels3::new(
        Size3::new(width as u16, depth as u16, slices.len() as u16),
        slices,
    ))
}

fn is_implicit_transparent_sprite_char(ch: char) -> bool {
    ch == '.' || ch == ' '
}

fn split_level_slices(rows: &[String]) -> Result<Vec<Vec<String>>, ParseError3> {
    let mut slices = Vec::<Vec<String>>::new();
    let mut current = Vec::<String>::new();
    for row in rows {
        if row.trim().is_empty() {
            if !current.is_empty() {
                slices.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(row.clone());
    }
    if !current.is_empty() {
        slices.push(current);
    }
    if slices.is_empty() {
        return Err(message("level requires at least one height slice"));
    }
    Ok(slices)
}

fn push_unique_object(objects: &mut Vec<ObjectId>, object: ObjectId) {
    if !objects.contains(&object) {
        objects.push(object);
    }
}

fn parse_lifecycle_command_line(line: &str) -> Result<LifecycleCommand3, ParseError3> {
    let command = line
        .strip_prefix("if win_conditions ->")
        .map(str::trim)
        .unwrap_or(line.trim());
    match command {
        "next_level" => Ok(LifecycleCommand3::NextLevel),
        _ => Err(message(format!("unknown lifecycle command: {line}"))),
    }
}

fn parse_rule_line(line: &str, catalog: &SelectorCatalog3) -> Result<Vec<Rule3>, ParseError3> {
    if let Some(effect) = parse_camera_rule_effect_line(line)? {
        return Ok(vec![
            Rule3::once(Pattern3::new(Vec::new()), Vec::new()).with_effects(vec![effect]),
        ]);
    }

    if let Some(rest) = line.strip_prefix("input ") {
        return parse_input_rule_line(rest.trim(), catalog);
    }

    let (prefix, rest) = line
        .split_once(' ')
        .ok_or_else(|| message("rule must be: <orientation> [ ... ] -> [ ... ]"))?;
    let (lhs, rhs, effects) = parse_rewrite(rest)?;
    if prefix.contains(':') || matches!(prefix, "frames" | "canonical" | "mirrored") {
        let orientation = parse_frame_orientation(prefix)?;
        let rule = DenseRuleTemplate3::once(
            orientation,
            parse_dense_pattern(lhs, catalog)?,
            infer_dense_writes(lhs, rhs, catalog)?,
        );
        let mut rules = lower_dense_rule_template(catalog, &rule)
            .map_err(|error| message(format!("failed to lower dense rule: {error:?}")))?;
        attach_rule_effects(&mut rules, &effects);
        return Ok(rules);
    }

    let orientation = parse_line_orientation(prefix)?;
    let rule = LineRuleTemplate3::once(
        orientation,
        parse_line_pattern(lhs, catalog)?,
        infer_line_writes(lhs, rhs, catalog)?,
    );
    let mut rules = lower_line_rule_template(catalog, &rule)
        .map_err(|error| message(format!("failed to lower line rule: {error:?}")))?;
    attach_rule_effects(&mut rules, &effects);
    Ok(rules)
}

fn attach_rule_effects(rules: &mut [Rule3], effects: &[RuleEffect3]) {
    if effects.is_empty() {
        return;
    }
    for rule in rules {
        rule.effects.extend(effects.iter().cloned());
    }
}

fn parse_input_rule_line(
    line: &str,
    catalog: &SelectorCatalog3,
) -> Result<Vec<Rule3>, ParseError3> {
    let mut rules = parse_rule_line(line, catalog)?;
    for rule in &mut rules {
        let direction = infer_input_direction(rule).ok_or_else(|| {
            message(format!(
                "input rule must lower to a directional pattern: {line}"
            ))
        })?;
        rule.guards
            .push(crate::Guard3::InputIs(input_for_direction(direction)));
    }
    Ok(rules)
}

fn infer_input_direction(rule: &Rule3) -> Option<Direction3> {
    for cell in &rule.pattern.cells {
        if let Some(direction) = direction_from_offset(cell.offset) {
            return Some(direction);
        }
    }
    for write in &rule.writes {
        match *write {
            crate::WriteOp3::Add { offset, .. } | crate::WriteOp3::Remove { offset, .. } => {
                if let Some(direction) = direction_from_offset(offset) {
                    return Some(direction);
                }
            }
            crate::WriteOp3::Replace { offset, .. } => {
                if let Some(direction) = direction_from_offset(offset) {
                    return Some(direction);
                }
            }
            crate::WriteOp3::Move {
                from_offset,
                to_offset,
                ..
            } => {
                if let Some(direction) = direction_from_offset(to_offset.add(from_offset.scale(-1)))
                {
                    return Some(direction);
                }
                if let Some(direction) = direction_from_offset(to_offset) {
                    return Some(direction);
                }
                if let Some(direction) = direction_from_offset(from_offset) {
                    return Some(direction);
                }
            }
        }
    }
    None
}

fn direction_from_offset(offset: Offset3) -> Option<Direction3> {
    if offset == Offset3::ZERO {
        return None;
    }
    Direction3::directions()
        .into_iter()
        .find(|direction| (1..=16).any(|step| offset == direction.offset.scale(step)))
}

fn input_for_direction(direction: Direction3) -> InputId3 {
    match direction.name {
        "left" => InputId3(0),
        "right" => InputId3(1),
        "up" => InputId3(2),
        "down" => InputId3(3),
        "forward" => InputId3(4),
        "backward" => InputId3(5),
        _ => unreachable!("built-in directions are exhaustive"),
    }
}

fn lower_win_conditions(
    catalog: &SelectorCatalog3,
    lines: &[String],
) -> Result<WinCondition3, ParseError3> {
    let conditions = lines
        .iter()
        .map(|line| parse_win_condition_line(line, catalog))
        .collect::<Result<Vec<_>, ParseError3>>()?;
    if conditions.is_empty() {
        return Err(message(
            "win_conditions block must contain at least one condition",
        ));
    }
    if conditions.len() == 1 {
        Ok(conditions.into_iter().next().unwrap())
    } else {
        Ok(WinCondition3::All(conditions))
    }
}

fn parse_win_condition_line(
    line: &str,
    catalog: &SelectorCatalog3,
) -> Result<WinCondition3, ParseError3> {
    if let Some((name, arg)) = parse_win_condition_call(line)? {
        return match name {
            "exists" | "some" => parse_some_condition(arg, catalog),
            "none" => parse_no_condition(arg, catalog),
            _ => Err(message(format!("unknown win condition function: {name}"))),
        };
    }
    if let Some(rest) = line.strip_prefix("some ") {
        return parse_some_condition(rest.trim(), catalog);
    }
    if let Some(rest) = line.strip_prefix("no ") {
        return parse_no_condition(rest.trim(), catalog);
    }
    if let Some(rest) = line.strip_prefix("all ") {
        return parse_all_on_condition(rest.trim(), catalog);
    }
    Err(message(format!("unknown win condition: {line}")))
}

fn parse_win_condition_call<'a>(line: &'a str) -> Result<Option<(&'a str, &'a str)>, ParseError3> {
    let Some((name, rest)) = line.split_once('(') else {
        return Ok(None);
    };
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Ok(None);
    }
    let Some(arg) = rest.strip_suffix(')') else {
        return Err(message(format!(
            "win condition function missing closing ): {line}"
        )));
    };
    Ok(Some((name, arg.trim())))
}

fn parse_some_condition(
    rest: &str,
    catalog: &SelectorCatalog3,
) -> Result<WinCondition3, ParseError3> {
    if rest.contains('[') {
        return pattern_conditions(rest, catalog, WinCondition3::SomePattern);
    }
    let objects = resolve_selector_objects(catalog, rest)?;
    Ok(any_object_condition(objects, WinCondition3::SomeObject))
}

fn parse_no_condition(
    rest: &str,
    catalog: &SelectorCatalog3,
) -> Result<WinCondition3, ParseError3> {
    if rest.contains('[') {
        return pattern_conditions(rest, catalog, WinCondition3::NoPattern);
    }
    let objects = resolve_selector_objects(catalog, rest)?;
    Ok(all_object_condition(objects, WinCondition3::NoObject))
}

fn parse_all_on_condition(
    rest: &str,
    catalog: &SelectorCatalog3,
) -> Result<WinCondition3, ParseError3> {
    let (object_selector, cover) = rest
        .split_once(" on ")
        .ok_or_else(|| message("all condition must be: all <selector> on <selector>"))?;
    let object = resolve_single_selector_object(catalog, object_selector.trim())?;
    if cover.contains('[') {
        return Err(message(
            "all <selector> on <pattern> is not valid; use some/no <orientation> [ ... ] for spatial win patterns",
        ));
    }
    let cover_object = resolve_single_selector_object(catalog, cover.trim())?;
    Ok(WinCondition3::NoPattern(Pattern3::new(vec![
        MatchCell3::new(Offset3::ZERO)
            .require(object)
            .forbid(cover_object),
    ])))
}

fn pattern_conditions(
    rest: &str,
    catalog: &SelectorCatalog3,
    wrap: fn(Pattern3) -> WinCondition3,
) -> Result<WinCondition3, ParseError3> {
    let patterns = parse_oriented_patterns(rest, catalog)?;
    if patterns.len() == 1 {
        Ok(wrap(patterns.into_iter().next().unwrap()))
    } else {
        Ok(WinCondition3::Any(patterns.into_iter().map(wrap).collect()))
    }
}

fn any_object_condition(
    objects: Vec<ObjectId>,
    wrap: fn(ObjectId) -> WinCondition3,
) -> WinCondition3 {
    if objects.len() == 1 {
        wrap(objects[0])
    } else {
        WinCondition3::Any(objects.into_iter().map(wrap).collect())
    }
}

fn all_object_condition(
    objects: Vec<ObjectId>,
    wrap: fn(ObjectId) -> WinCondition3,
) -> WinCondition3 {
    if objects.len() == 1 {
        wrap(objects[0])
    } else {
        WinCondition3::All(objects.into_iter().map(wrap).collect())
    }
}

fn resolve_selector_objects(
    catalog: &SelectorCatalog3,
    token: &str,
) -> Result<Vec<ObjectId>, ParseError3> {
    let selector = parse_selector(token, &catalog.families, &catalog.groups)?;
    catalog
        .resolve(&selector)
        .map(|resolved| resolved.alternatives)
        .map_err(|error| message(format!("invalid win selector: {error:?}")))
}

fn resolve_single_selector_object(
    catalog: &SelectorCatalog3,
    token: &str,
) -> Result<ObjectId, ParseError3> {
    let objects = resolve_selector_objects(catalog, token)?;
    if objects.len() != 1 {
        return Err(message(format!(
            "win selector must resolve to one object: {token}"
        )));
    }
    Ok(objects[0])
}

fn parse_oriented_patterns(
    value: &str,
    catalog: &SelectorCatalog3,
) -> Result<Vec<Pattern3>, ParseError3> {
    let (prefix, rest) = value
        .split_once(' ')
        .ok_or_else(|| message("pattern condition must be: <orientation> [ ... ]"))?;
    let inner = parse_bracketed(rest.trim())?;
    if prefix.contains(':') || matches!(prefix, "frames" | "canonical" | "mirrored") {
        let orientation = parse_frame_orientation(prefix)?;
        let rule = DenseRuleTemplate3::once(
            orientation,
            parse_dense_pattern(inner, catalog)?,
            Vec::new(),
        );
        return lower_dense_rule_template(catalog, &rule)
            .map(|rules| rules.into_iter().map(|rule| rule.pattern).collect())
            .map_err(|error| message(format!("failed to lower win pattern: {error:?}")));
    }
    let orientation = parse_line_orientation(prefix)?;
    let rule =
        LineRuleTemplate3::once(orientation, parse_line_pattern(inner, catalog)?, Vec::new());
    lower_line_rule_template(catalog, &rule)
        .map(|rules| rules.into_iter().map(|rule| rule.pattern).collect())
        .map_err(|error| message(format!("failed to lower win pattern: {error:?}")))
}

fn parse_rewrite(rest: &str) -> Result<(&str, &str, Vec<RuleEffect3>), ParseError3> {
    let (lhs, rhs) = rest
        .split_once("->")
        .ok_or_else(|| message("rewrite missing ->"))?;
    let (rhs, suffix) = parse_bracketed_with_suffix(rhs.trim())?;
    let effects = parse_rule_effect_suffix(suffix)?;
    Ok((parse_bracketed(lhs.trim())?, rhs, effects))
}

fn parse_bracketed(value: &str) -> Result<&str, ParseError3> {
    value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .map(str::trim)
        .ok_or_else(|| message("pattern must be enclosed in [ ]"))
}

fn parse_bracketed_with_suffix(value: &str) -> Result<(&str, &str), ParseError3> {
    let value = value.trim();
    let Some(rest) = value.strip_prefix('[') else {
        return Err(message("pattern must be enclosed in [ ]"));
    };
    let Some(end) = rest.find(']') else {
        return Err(message("pattern must be enclosed in [ ]"));
    };
    Ok((rest[..end].trim(), rest[end + 1..].trim()))
}

fn parse_rule_effect_suffix(suffix: &str) -> Result<Vec<RuleEffect3>, ParseError3> {
    if suffix.is_empty() {
        return Ok(Vec::new());
    }
    let Some(effect) = parse_camera_rule_effect_line(suffix)? else {
        return Err(message(format!("unknown 3D rule effect: {suffix}")));
    };
    Ok(vec![effect])
}

fn parse_camera_rule_effect_line(line: &str) -> Result<Option<RuleEffect3>, ParseError3> {
    if line.trim() == "reset_camera" {
        return Ok(Some(RuleEffect3::ResetCamera));
    }
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    let ["set", name, "=", value] = tokens.as_slice() else {
        return Ok(None);
    };
    match *name {
        "yaw" => Ok(Some(RuleEffect3::SetCameraYaw(parse_degrees_setting(
            value, name,
        )?))),
        "pitch" => Ok(Some(RuleEffect3::SetCameraPitch(parse_degrees_setting(
            value, name,
        )?))),
        "zoom" => Ok(Some(RuleEffect3::SetCameraZoom(parse_zoom_milli_setting(
            value, name,
        )?))),
        _ => Err(message(format!("unknown 3D view variable: {name}"))),
    }
}

fn parse_line_orientation(prefix: &str) -> Result<LineOrientation3, ParseError3> {
    if let Some(direction) = Direction3::by_name(prefix) {
        return Ok(LineOrientation3::Direction(direction));
    }
    let set = parse_direction_set(prefix)?;
    Ok(LineOrientation3::DirectionSet(set))
}

fn parse_frame_orientation(prefix: &str) -> Result<FrameOrientation3, ParseError3> {
    match prefix {
        "frames" => return Ok(FrameOrientation3::FrameSet(crate::FrameSet3::Frames)),
        "canonical" => return Ok(FrameOrientation3::FrameSet(crate::FrameSet3::Canonical)),
        "mirrored" => return Ok(FrameOrientation3::FrameSet(crate::FrameSet3::Mirrored)),
        _ => {}
    }
    let parts = prefix.split(':').collect::<Vec<_>>();
    if parts.len() < 2 || parts.len() > 3 {
        return Err(message("frame orientation must be A:B or A:B:C"));
    }
    let expr = if parts.len() == 2 {
        crate::FrameExpr3::from_two(parse_frame_slot(parts[0])?, parse_frame_slot(parts[1])?)
    } else {
        crate::FrameExpr3::new(
            parse_frame_slot(parts[0])?,
            parse_frame_slot(parts[1])?,
            parse_frame_slot(parts[2])?,
        )
    };
    let frames = expr.expand();
    if frames.len() == 1 {
        Ok(FrameOrientation3::Frame(frames[0]))
    } else {
        Ok(FrameOrientation3::Frames(frames))
    }
}

fn parse_frame_slot(value: &str) -> Result<FrameSlot3, ParseError3> {
    if value == "_" {
        return Ok(FrameSlot3::CompleteCanonical);
    }
    if let Some(direction) = Direction3::by_name(value) {
        return Ok(FrameSlot3::Direction(direction));
    }
    Ok(FrameSlot3::DirectionSet(parse_direction_set(value)?))
}

fn parse_direction_set(value: &str) -> Result<DirectionSet3, ParseError3> {
    match value {
        "directions" => Ok(DirectionSet3::Directions),
        "horizontal" => Ok(DirectionSet3::Horizontal),
        "vertical" => Ok(DirectionSet3::Vertical),
        _ => Err(message(format!("unknown direction set: {value}"))),
    }
}

fn parse_line_pattern(
    inner: &str,
    catalog: &SelectorCatalog3,
) -> Result<LinePatternTemplate3, ParseError3> {
    Ok(LinePatternTemplate3::new(
        split_line_cells(inner)
            .into_iter()
            .enumerate()
            .map(|(step, cell)| {
                let parsed = parse_cell(cell, catalog)?;
                Ok(LineMatchCellTemplate3 {
                    step: step as i16,
                    require: parsed.require,
                    forbid: parsed.forbid,
                })
            })
            .collect::<Result<Vec<_>, ParseError3>>()?,
    ))
}

fn parse_dense_pattern(
    inner: &str,
    catalog: &SelectorCatalog3,
) -> Result<DensePattern3, ParseError3> {
    Ok(DensePattern3::new(
        inner
            .split(";;")
            .map(str::trim)
            .filter(|slice| !slice.is_empty())
            .map(|slice| {
                Ok(DenseSlice3::new(
                    slice
                        .split(';')
                        .map(str::trim)
                        .map(|row| {
                            Ok(DenseRow3::new(
                                split_line_cells(row)
                                    .into_iter()
                                    .map(|cell| {
                                        let parsed = parse_cell(cell, catalog)?;
                                        Ok(DenseCell3 {
                                            require: parsed.require,
                                            forbid: parsed.forbid,
                                        })
                                    })
                                    .collect::<Result<Vec<_>, ParseError3>>()?,
                            ))
                        })
                        .collect::<Result<Vec<_>, ParseError3>>()?,
                ))
            })
            .collect::<Result<Vec<_>, ParseError3>>()?,
    ))
}

fn split_line_cells(inner: &str) -> Vec<&str> {
    inner.split('|').map(str::trim).collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ParsedCell3 {
    require: Vec<ObjectSelector3>,
    forbid: Vec<ObjectSelector3>,
}

fn parse_cell(cell: &str, catalog: &SelectorCatalog3) -> Result<ParsedCell3, ParseError3> {
    let mut require = Vec::new();
    let mut forbid = Vec::new();
    let tokens = cell.split_whitespace().collect::<Vec<_>>();
    let mut index = 0;
    while index < tokens.len() {
        if tokens[index] == "no" {
            let selector = tokens
                .get(index + 1)
                .ok_or_else(|| message("no must be followed by a selector"))?;
            forbid.push(parse_selector(
                selector,
                &catalog.families,
                &catalog.groups,
            )?);
            index += 2;
        } else {
            require.push(parse_selector(
                tokens[index],
                &catalog.families,
                &catalog.groups,
            )?);
            index += 1;
        }
    }
    Ok(ParsedCell3 { require, forbid })
}

fn infer_line_writes(
    lhs: &str,
    rhs: &str,
    catalog: &SelectorCatalog3,
) -> Result<Vec<LineWriteOpTemplate3>, ParseError3> {
    let before = parse_positive_line_cells(lhs, catalog)?;
    let after = parse_positive_line_cells(rhs, catalog)?;
    infer_moves(before, after)
        .into_iter()
        .map(|write| match write {
            InferredWrite3::Add { to, object } => {
                Ok(LineWriteOpTemplate3::Add { step: to.0, object })
            }
            InferredWrite3::Remove { from, object } => Ok(LineWriteOpTemplate3::Remove {
                step: from.0,
                object,
            }),
            InferredWrite3::Move { from, to, object } => Ok(LineWriteOpTemplate3::Move {
                from_step: from.0,
                to_step: to.0,
                object,
            }),
        })
        .collect()
}

fn infer_dense_writes(
    lhs: &str,
    rhs: &str,
    catalog: &SelectorCatalog3,
) -> Result<Vec<LocalWriteOpTemplate3>, ParseError3> {
    let before = parse_positive_dense_cells(lhs, catalog)?;
    let after = parse_positive_dense_cells(rhs, catalog)?;
    Ok(infer_moves(before, after)
        .into_iter()
        .map(|write| match write {
            InferredWrite3::Add { to, object } => LocalWriteOpTemplate3::Add {
                offset: to.1,
                object,
            },
            InferredWrite3::Remove { from, object } => LocalWriteOpTemplate3::Remove {
                offset: from.1,
                object,
            },
            InferredWrite3::Move { from, to, object } => LocalWriteOpTemplate3::Move {
                from_offset: from.1,
                to_offset: to.1,
                object,
            },
        })
        .collect())
}

type LinePos3 = (i16, Offset3);

#[derive(Clone, Debug, PartialEq, Eq)]
enum InferredWrite3 {
    Add {
        to: LinePos3,
        object: ObjectSelector3,
    },
    Remove {
        from: LinePos3,
        object: ObjectSelector3,
    },
    Move {
        from: LinePos3,
        to: LinePos3,
        object: ObjectSelector3,
    },
}

fn infer_moves(
    before: Vec<(LinePos3, ObjectSelector3)>,
    after: Vec<(LinePos3, ObjectSelector3)>,
) -> Vec<InferredWrite3> {
    let mut writes = Vec::new();
    let mut used_after = vec![false; after.len()];
    for (from, object) in &before {
        let token = object.token();
        let Some(index) = after
            .iter()
            .enumerate()
            .find_map(|(index, (_, after_object))| {
                (!used_after[index] && after_object.token() == token).then_some(index)
            })
        else {
            writes.push(InferredWrite3::Remove {
                from: *from,
                object: object.clone(),
            });
            continue;
        };
        used_after[index] = true;
        let to = after[index].0;
        if *from != to {
            writes.push(InferredWrite3::Move {
                from: *from,
                to,
                object: object.clone(),
            });
        }
    }
    for (index, (to, object)) in after.into_iter().enumerate() {
        if !used_after[index] {
            writes.push(InferredWrite3::Add { to, object });
        }
    }
    writes
}

fn parse_positive_line_cells(
    inner: &str,
    catalog: &SelectorCatalog3,
) -> Result<Vec<(LinePos3, ObjectSelector3)>, ParseError3> {
    let mut parsed = Vec::new();
    for (step, cell) in split_line_cells(inner).into_iter().enumerate() {
        for selector in parse_cell(cell, catalog)?.require {
            parsed.push(((step as i16, Offset3::new(step as i16, 0, 0)), selector));
        }
    }
    Ok(parsed)
}

fn parse_positive_dense_cells(
    inner: &str,
    catalog: &SelectorCatalog3,
) -> Result<Vec<(LinePos3, ObjectSelector3)>, ParseError3> {
    let mut parsed = Vec::new();
    for (depth, slice) in inner.split(";;").map(str::trim).enumerate() {
        for (row, row_text) in slice.split(';').map(str::trim).enumerate() {
            for (column, cell) in split_line_cells(row_text).into_iter().enumerate() {
                for selector in parse_cell(cell, catalog)?.require {
                    let local = Offset3::new(column as i16, row as i16, depth as i16);
                    parsed.push(((column as i16, local), selector));
                }
            }
        }
    }
    Ok(parsed)
}

fn parse_selector(
    token: &str,
    families: &[ObjectFamily3],
    groups: &[SelectorGroup3],
) -> Result<ObjectSelector3, ParseError3> {
    let parts = token.split(':').collect::<Vec<_>>();
    if parts.len() > 1 {
        return Ok(ObjectSelector3::variant(
            parts[0],
            parts[1..]
                .iter()
                .map(|part| {
                    if *part == "*" {
                        SelectorTag3::any()
                    } else {
                        SelectorTag3::value(*part)
                    }
                })
                .collect(),
        ));
    }
    if families.iter().any(|family| family.name == token) {
        return Err(message(format!(
            "variant selector must use explicit tags: {token}"
        )));
    }
    if groups.iter().any(|group| group.name == token) {
        return Ok(ObjectSelector3::group(token));
    }
    Ok(ObjectSelector3::object(token))
}

fn cartesian_values(axes: &[Vec<String>]) -> Vec<Vec<String>> {
    let mut values = vec![Vec::new()];
    for axis in axes {
        let mut next = Vec::new();
        for prefix in &values {
            for value in axis {
                let mut row = prefix.clone();
                row.push(value.clone());
                next.push(row);
            }
        }
        values = next;
    }
    values
}

fn direction_names(set: DirectionSet3) -> Vec<String> {
    set.directions()
        .into_iter()
        .map(|direction| direction.name.to_string())
        .collect()
}

fn default_inputs() -> Vec<InputDef3> {
    vec![
        InputDef3::directional(InputId3(0), "left", Direction3::LEFT),
        InputDef3::directional(InputId3(1), "right", Direction3::RIGHT),
        InputDef3::directional(InputId3(2), "up", Direction3::UP),
        InputDef3::directional(InputId3(3), "down", Direction3::DOWN),
        InputDef3::directional(InputId3(4), "forward", Direction3::FORWARD),
        InputDef3::directional(InputId3(5), "backward", Direction3::BACKWARD),
    ]
}

fn inputs_from_specs(specs: Vec<InputSpec3>) -> Result<Vec<InputDef3>, ParseError3> {
    if specs.is_empty() {
        return Ok(default_inputs());
    }

    let defaults = default_inputs();
    let mut next_id = defaults
        .iter()
        .map(|input| input.id.0)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let mut inputs = Vec::new();
    for spec in specs {
        if inputs
            .iter()
            .any(|input: &InputDef3| input.name == spec.name)
        {
            return Err(message(format!("duplicate input: {}", spec.name)));
        }
        let input = if let Some(default) = defaults.iter().find(|input| input.name == spec.name) {
            default.clone().with_keys(spec.keys)
        } else {
            let id = InputId3(next_id);
            next_id = next_id.saturating_add(1);
            InputDef3::action(id, spec.name).with_keys(spec.keys)
        };
        inputs.push(input);
    }
    Ok(inputs)
}

fn strip_comment(line: &str) -> &str {
    line.split_once("//").map_or(line, |(head, _)| head)
}

fn message(message: impl Into<String>) -> ParseError3 {
    ParseError3::Message(message.into())
}
