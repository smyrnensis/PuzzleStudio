use std::collections::{BTreeMap, HashMap};

use crate::{
    DenseCell3, DensePattern3, DenseRow3, DenseRuleTemplate3, DenseSlice3, Direction3,
    DirectionSet3, FrameOrientation3, FrameSlot3, Game3, InputDef3, InputId3, LayerId, Level3,
    LevelBundle3, LevelCell3, LevelEntry3, Lifecycle3, LifecycleCommand3, LineMatchCellTemplate3,
    LineOrientation3, LinePatternTemplate3, LineRuleTemplate3, LineWriteOpTemplate3,
    LocalWriteOpTemplate3, MatchCell3, ObjectDef3, ObjectFamily3, ObjectId, ObjectSelector3,
    ObjectSetMatcher3, ObjectVariant3, Offset3, Pattern3, Rule3, RuleEffect3, ScratchId3,
    SelectorCatalog3, SelectorGroup3, SelectorScratch3, SelectorTag3, Size3, Sprite3, SpriteColor3,
    SpriteSet3, SpriteVoxels3, VariantAxis3, WinCondition3, WriteOp3, lower_dense_rule_template,
    lower_line_rule_template,
};
use puzzle_kernel::{LocalFrame, LocalFrameExtent};

const DEFAULT_LINE_GAP_LIMIT3: u16 = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedPuzzle3 {
    pub game: Game3,
    pub catalog: SelectorCatalog3,
    pub settings: ModelSettings3,
    pub local_frame: Option<LocalFrame<ObjectId>>,
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
    pub viewport: ViewportSettings3,
    pub pixelate: PixelateRenderSettings3,
}

impl Default for ModelSettings3 {
    fn default() -> Self {
        Self {
            camera: CameraSettings3::default(),
            grid: GridSettings3::default(),
            sprite: SpriteRenderSettings3::default(),
            viewport: ViewportSettings3::default(),
            pixelate: PixelateRenderSettings3::default(),
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
pub struct PixelateRenderSettings3 {
    pub enabled: bool,
    pub scale: u16,
    pub smoothing: bool,
}

impl Default for PixelateRenderSettings3 {
    fn default() -> Self {
        Self {
            enabled: false,
            scale: 4,
            smoothing: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewportSettings3 {
    pub mode: ViewportMode3,
    pub follow: ViewportFollow3,
    pub framing: Option<ViewportFraming3>,
    pub focus: String,
}

impl Default for ViewportSettings3 {
    fn default() -> Self {
        Self {
            mode: ViewportMode3::Full,
            follow: ViewportFollow3::Snap,
            framing: None,
            focus: "Player".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportMode3 {
    Full,
    Centered,
    Paged,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportFollow3 {
    Snap,
    Smooth,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewportFraming3 {
    pub width: u16,
    pub depth: u16,
    pub height: ViewportHeight3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportHeight3 {
    Full,
    Size(u16),
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
    local_frame_modifier: Option<String>,
    on_level_start_lines: Vec<String>,
    on_level_start_local_frame_modifier: Option<String>,
    on_level_clear_lines: Vec<String>,
    on_last_level_clear_lines: Option<Vec<String>>,
    win_condition_lines: Vec<String>,
    settings: ModelSettings3,
    sprite_set: Option<SpriteSet3>,
}

impl Parser3 {
    fn new(source: &str) -> Self {
        Self {
            lines: preprocess_source_lines3(source),
            value_sets: Vec::new(),
            input_specs: Vec::new(),
            layers: Vec::new(),
            object_specs: Vec::new(),
            group_specs: Vec::new(),
            legend_specs: Vec::new(),
            level_specs: Vec::new(),
            rule_lines: Vec::new(),
            local_frame_modifier: None,
            on_level_start_lines: Vec::new(),
            on_level_start_local_frame_modifier: None,
            on_level_clear_lines: Vec::new(),
            on_last_level_clear_lines: None,
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
            } else if line == "display_objects {" {
                return Err(message(
                    "`display_objects { ... }` was removed; use `objects { @Name layer }`",
                ));
            } else if line == "keys {" {
                index = self.parse_keys_block(index + 1)?;
            } else if line == "inputs {" {
                return Err(message(
                    "`inputs { ... }` was removed; use `keys { <key...> -> <input> }`",
                ));
            } else if line == "groups {" {
                index = self.parse_groups_block(index + 1)?;
            } else if line == "group {" {
                return Err(message("`group { ... }` was removed; use `groups { ... }`"));
            } else if line == "legend {" {
                index = self.parse_legend_block(index + 1)?;
            } else if is_levels3_header(&line) {
                index = self.parse_levels_block(index + 1)?;
            } else if is_sprites3_header(&line) {
                index = self.parse_sprites3_block(index + 1, &line)?;
            } else if let Some(name) = parse_scene_header(&line) {
                let _ = name;
                index = skip_braced_block(&self.lines, index + 1)?;
            } else if let Some(block) = puzzle_authoring::rule_program_block_surface(&line) {
                index = self.parse_rule_program_block(block, index + 1)?;
            } else if line == "win_conditions {" {
                index = self.parse_win_conditions_block(index + 1)?;
            } else if line == "render {" {
                index = self.parse_render_block(index + 1)?;
            } else if let Some(rest) = line.strip_prefix("group ") {
                self.group_specs.push(parse_group_spec(rest)?);
                index += 1;
            } else if legacy_model_setting_name(&line).is_some() {
                return Err(message(format!(
                    "legacy model setting is not supported: {line}"
                )));
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
        let visual_objects = build.visual_objects.clone();
        let catalog = build.catalog_with_groups(self.group_specs)?;
        let game = Game3::new_with_inputs_and_roles(
            layer_count,
            object_defs,
            inputs_from_specs(self.input_specs)?,
            visual_objects,
            Vec::new(),
        );
        let local_frame =
            parse_optional_program_local_frame(self.local_frame_modifier.as_deref(), &catalog)?;
        let on_level_start_local_frame = parse_optional_program_local_frame(
            self.on_level_start_local_frame_modifier.as_deref(),
            &catalog,
        )?;

        let line_gap_limit = line_gap_limit_from_levels(&self.level_specs);
        let mut rules = Vec::new();
        for line in &self.rule_lines {
            rules.extend(parse_rule_line(line, &catalog, &game, line_gap_limit)?);
        }
        let mut on_level_start = Vec::new();
        for line in &self.on_level_start_lines {
            on_level_start.extend(parse_rule_line(line, &catalog, &game, line_gap_limit)?);
        }
        let on_level_clear = self
            .on_level_clear_lines
            .iter()
            .map(|line| parse_lifecycle_command_line(line))
            .collect::<Result<Vec<_>, ParseError3>>()?;
        let on_last_level_clear = self
            .on_last_level_clear_lines
            .as_ref()
            .map(|lines| {
                lines
                    .iter()
                    .map(|line| parse_lifecycle_command_line(line))
                    .collect::<Result<Vec<_>, ParseError3>>()
            })
            .transpose()?;
        let mut lifecycle = Lifecycle3::new(on_level_start, on_level_clear);
        lifecycle.on_level_start_local_frame = on_level_start_local_frame;
        lifecycle.on_last_level_clear = on_last_level_clear;
        let win_condition = if self.win_condition_lines.is_empty() {
            None
        } else {
            Some(lower_win_conditions(
                &catalog,
                &self.win_condition_lines,
                line_gap_limit,
            )?)
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
            local_frame,
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
            } else if line == "display_objects {" {
                return Err(message(
                    "`display_objects { ... }` was removed; use `objects { @Name layer }`",
                ));
            } else if line == "keys {" {
                index = self.parse_keys_block(index + 1)?;
            } else if line == "inputs {" {
                return Err(message(
                    "`inputs { ... }` was removed; use `keys { <key...> -> <input> }`",
                ));
            } else if line == "groups {" {
                index = self.parse_groups_block(index + 1)?;
            } else if line == "group {" {
                return Err(message("`group { ... }` was removed; use `groups { ... }`"));
            } else if let Some(block) = puzzle_authoring::rule_program_block_surface(&line) {
                index = self.parse_rule_program_block(block, index + 1)?;
            } else if line == "win_conditions {" {
                index = self.parse_win_conditions_block(index + 1)?;
            } else if line == "render {" {
                index = self.parse_render_block(index + 1)?;
            } else if is_sprites3_header(&line) {
                index = self.parse_sprites3_block(index + 1, &line)?;
            } else if let Some(name) = parse_scene_header(&line) {
                let _ = name;
                index = skip_braced_block(&self.lines, index + 1)?;
            } else if let Some(rest) = line.strip_prefix("group ") {
                self.group_specs.push(parse_group_spec(rest)?);
                index += 1;
            } else if legacy_model_setting_name(&line).is_some() {
                return Err(message(format!(
                    "legacy model setting is not supported: {line}"
                )));
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
            ModelSetting3::CameraYaw(value) => self.settings.camera.yaw_degrees = value,
            ModelSetting3::CameraPitch(value) => self.settings.camera.pitch_degrees = value,
            ModelSetting3::CameraZoom(value) => self.settings.camera.zoom_milli = value,
            ModelSetting3::InteractiveLook => self.settings.camera.interactive_look = true,
            ModelSetting3::InteractiveZoom => self.settings.camera.interactive_zoom = true,
            ModelSetting3::OccupiedCellGrid => self.settings.grid.occupied_cells = true,
            ModelSetting3::SpriteShade => self.settings.sprite.shade = true,
            ModelSetting3::PixelateScale(value) => self.settings.pixelate.scale = value,
            ModelSetting3::PixelateSmoothing => self.settings.pixelate.smoothing = true,
        }
    }

    fn apply_model_settings(&mut self, settings: Vec<ModelSetting3>) {
        for setting in settings {
            self.apply_model_setting(setting);
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
            } else if let Some(rest) = line.strip_prefix("camera ") {
                let settings = parse_camera_inline(rest)?;
                self.apply_model_settings(settings);
                index += 1;
            } else if line == "grid {" {
                index = self.parse_grid_block(index + 1)?;
            } else if let Some(rest) = line.strip_prefix("grid ") {
                let settings = parse_grid_inline(rest)?;
                self.apply_model_settings(settings);
                index += 1;
            } else if line == "pixelate" {
                self.settings.pixelate.enabled = true;
                index += 1;
            } else if line == "pixelate {" {
                self.settings.pixelate.enabled = true;
                index = self.parse_pixelate_block(index + 1)?;
            } else if let Some(rest) = line.strip_prefix("pixelate ") {
                self.settings.pixelate.enabled = true;
                let settings = parse_pixelate_inline(rest)?;
                self.apply_model_settings(settings);
                index += 1;
            } else if line == "viewport {" {
                index = self.parse_viewport_block(index + 1)?;
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

    fn parse_viewport_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = self.lines[index].clone();
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            parse_viewport_directive(&line, &mut self.settings.viewport)?;
            index += 1;
        }
        Err(message("viewport block missing }"))
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

    fn parse_pixelate_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = self.lines[index].clone();
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            let setting = parse_pixelate_setting_line(&line)?;
            self.apply_model_setting(setting);
            index += 1;
        }
        Err(message("pixelate block missing }"))
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
        let mut shapes = HashMap::<String, Vec<String>>::new();
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
                    "sprites3 entries must use canonical form: <name>, color row, voxel rows or shape ref",
                ));
            }
            if let Some(shape_name) = parse_sprite3_shape_header(&line) {
                if shapes.contains_key(shape_name) {
                    return Err(message(format!("duplicate sprite3 shape: {shape_name}")));
                }
                let (next, rows) = parse_sprite3_shape_block(&self.lines, index + 1, shape_name)?;
                shapes.insert(shape_name.to_string(), rows);
                index = next;
                continue;
            }
            if is_canonical_sprite_name(&line) {
                let sprite_name = line.clone();
                if sprites.iter().any(|sprite| sprite.name == sprite_name) {
                    return Err(message(format!("duplicate sprite: {sprite_name}")));
                }
                let (next, sprite) =
                    self.parse_canonical_sprite(index + 1, sprite_name, &shapes)?;
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
        shapes: &HashMap<String, Vec<String>>,
    ) -> Result<(usize, Sprite3), ParseError3> {
        while index < self.lines.len() && self.lines[index].is_empty() {
            index += 1;
        }
        if index >= self.lines.len() || self.lines[index] == "}" {
            return Err(message(format!("sprite {name} missing color row")));
        }
        let palette = parse_canonical_sprite_palette_line(&self.lines[index])?;
        index += 1;

        while index < self.lines.len() && self.lines[index].is_empty() {
            index += 1;
        }

        if index < self.lines.len() {
            let line = self.lines[index].clone();
            if line == "}"
                || self.is_canonical_sprite_start(index)
                || parse_sprite3_shape_header(&line).is_some()
            {
                let rows = vec!["0".to_string()];
                let voxels = parse_sprite_voxels(&name, &rows, &palette)?;
                return Ok((index, Sprite3::new(name, palette, voxels)));
            }
            if let Some(rows) = shapes.get(&line) {
                let voxels = parse_sprite_voxels(&name, rows, &palette)?;
                return Ok((index + 1, Sprite3::new(name, palette, voxels)));
            }
            if let Some(rest) = line.strip_prefix("shape ") {
                let shape_name = rest.trim();
                if shapes.contains_key(shape_name) {
                    return Err(message("sprite3 shape refs are bare; remove `shape`"));
                }
            }
        }

        let mut rows = Vec::new();
        while index < self.lines.len() {
            let line = self.lines[index].clone();
            if line == "}" {
                break;
            }
            if line.starts_with("sprite ") || line == "colors {" || line == "voxels {" {
                return Err(message(
                    "sprites3 entries must use canonical form: <name>, color row, voxel rows or shape ref",
                ));
            }
            if !rows.is_empty()
                && (self.is_canonical_sprite_start(index)
                    || parse_sprite3_shape_header(&line).is_some())
            {
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
        let mut has_anonymous_layer_rows = false;
        while index < self.lines.len() {
            let line = self.lines[index].clone();
            if line == "}" {
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            if self.parse_layer_line(&line, has_anonymous_layer_rows)? {
                has_anonymous_layer_rows = true;
            }
            index += 1;
        }
        Err(message("layers block missing }"))
    }

    fn parse_layer_line(
        &mut self,
        line: &str,
        has_anonymous_layer_rows: bool,
    ) -> Result<bool, ParseError3> {
        if let Some((layer, objects)) = line.split_once('=') {
            if has_anonymous_layer_rows {
                self.group_specs.push(parse_group_spec(line)?);
                return Ok(false);
            }
            let layer = layer.trim();
            if layer.is_empty() {
                return Err(message("layer declaration must name a layer before ="));
            }
            reject_occurrence_label_marker_in_name(layer, "layer name")?;
            self.layers.push(layer.to_string());
            for object in objects.split_whitespace() {
                self.object_specs
                    .push(parse_layer_object_spec(object, layer)?);
            }
            return Ok(false);
        }
        for layer in line.split_whitespace() {
            reject_occurrence_label_marker_in_name(layer, "layer name")?;
            self.layers.push(layer.to_string());
        }
        Ok(true)
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

    fn parse_keys_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        while index < self.lines.len() {
            let line = &self.lines[index];
            if line == "}" {
                return Ok(index + 1);
            }
            if !line.is_empty() {
                self.input_specs.push(parse_key_input_spec(line)?);
            }
            index += 1;
        }
        Err(message("keys block missing }"))
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

    fn parse_rule_program_block(
        &mut self,
        block: puzzle_authoring::RuleProgramBlockSurface<'_>,
        start: usize,
    ) -> Result<usize, ParseError3> {
        match block {
            puzzle_authoring::RuleProgramBlockSurface::Rules { modifier } => {
                self.local_frame_modifier =
                    (!modifier.trim().is_empty()).then(|| modifier.trim().to_string());
                self.parse_rules_block(start)
            }
            puzzle_authoring::RuleProgramBlockSurface::OnLevelStart { modifier } => {
                self.on_level_start_local_frame_modifier =
                    (!modifier.trim().is_empty()).then(|| modifier.trim().to_string());
                self.parse_on_level_start_block(start)
            }
            puzzle_authoring::RuleProgramBlockSurface::OnLevelClear => {
                self.parse_on_level_clear_block(start)
            }
            puzzle_authoring::RuleProgramBlockSurface::OnLastLevelClear => {
                self.parse_on_last_level_clear_block(start)
            }
        }
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
            let (rule_line, next_index) = collect_multiline_rule_line(&self.lines, index)?;
            self.rule_lines.push(rule_line);
            index = next_index;
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

    fn parse_on_last_level_clear_block(&mut self, mut index: usize) -> Result<usize, ParseError3> {
        if self.on_last_level_clear_lines.is_some() {
            return Err(message(
                "multiple last_level_clear blocks are not supported",
            ));
        }
        let mut lines = Vec::new();
        while index < self.lines.len() {
            let line = &self.lines[index];
            if line == "}" {
                self.on_last_level_clear_lines = Some(lines);
                return Ok(index + 1);
            }
            if line.is_empty() {
                index += 1;
                continue;
            }
            lines.push(line.clone());
            index += 1;
        }
        Err(message("on_last_level_clear block missing }"))
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
    visual: bool,
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

fn line_gap_limit_from_levels(level_specs: &[LevelSpec3]) -> u16 {
    level_specs
        .iter()
        .filter_map(|spec| split_level_slices(&spec.rows).ok())
        .filter_map(|slices| {
            let height = slices.len();
            let depth = slices.first()?.len();
            let width = slices.first()?.first()?.chars().count();
            [width, depth, height].into_iter().max()
        })
        .max()
        .and_then(|value| u16::try_from(value.saturating_sub(1)).ok())
        .unwrap_or(DEFAULT_LINE_GAP_LIMIT3)
}

struct CatalogBuild3 {
    value_sets: Vec<(String, Vec<String>)>,
    layers: Vec<String>,
    next_object: u16,
    concrete: Vec<crate::ConcreteObject3>,
    families: Vec<ObjectFamily3>,
    object_defs: Vec<ObjectDef3>,
    visual_objects: Vec<ObjectId>,
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
            visual_objects: Vec::new(),
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
            if spec.visual {
                push_unique_object(&mut self.visual_objects, id);
            }
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
            if spec.visual {
                push_unique_object(&mut self.visual_objects, id);
            }
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
        let object_layers = self
            .object_defs
            .iter()
            .map(|def| (def.id, def.layer_id))
            .collect();
        SelectorCatalog3::checked_new_with_object_layers(
            self.concrete,
            self.families,
            groups,
            object_layers,
        )
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

fn legacy_model_setting_name(line: &str) -> Option<&str> {
    let (name, _) = line.split_once('=')?;
    match name.trim() {
        "debug_camera" | "camera_yaw" | "camera_pitch" | "camera_zoom" => Some(name.trim()),
        _ => None,
    }
}

enum ModelSetting3 {
    CameraYaw(i16),
    CameraPitch(i16),
    CameraZoom(u16),
    InteractiveLook,
    InteractiveZoom,
    OccupiedCellGrid,
    SpriteShade,
    PixelateScale(u16),
    PixelateSmoothing,
}

pub const RENDER_BLOCK_OPTIONS3: &[&str] = &["camera", "grid", "pixelate", "viewport"];
pub const RENDER_BARE_OPTIONS3: &[&str] = &["shade", "pixelate"];
pub const RENDER_OPTIONS3: &[&str] = &["camera", "grid", "pixelate", "viewport", "shade"];
pub const CAMERA_ASSIGNMENT_OPTIONS3: &[&str] = &["yaw", "pitch", "zoom"];
pub const CAMERA_BARE_OPTIONS3: &[&str] = &["interactive_look", "interactive_zoom"];
pub const CAMERA_OPTIONS3: &[&str] = &[
    "yaw",
    "pitch",
    "zoom",
    "interactive_look",
    "interactive_zoom",
];
pub const GRID_BARE_OPTIONS3: &[&str] = &["occupied_cells"];
pub const PIXELATE_ASSIGNMENT_OPTIONS3: &[&str] = &["scale"];
pub const PIXELATE_BARE_OPTIONS3: &[&str] = &["smoothing"];
pub const PIXELATE_OPTIONS3: &[&str] = &["scale", "smoothing"];

fn parse_camera_setting_line(line: &str) -> Result<ModelSetting3, ParseError3> {
    if line == CAMERA_BARE_OPTIONS3[0] {
        return Ok(ModelSetting3::InteractiveLook);
    }
    if line == CAMERA_BARE_OPTIONS3[1] {
        return Ok(ModelSetting3::InteractiveZoom);
    }
    let (name, value) = parse_setting_assignment(line, "camera setting")?;
    match name {
        name if name == CAMERA_ASSIGNMENT_OPTIONS3[0] => Ok(ModelSetting3::CameraYaw(
            parse_degrees_setting(value, name)?,
        )),
        name if name == CAMERA_ASSIGNMENT_OPTIONS3[1] => Ok(ModelSetting3::CameraPitch(
            parse_degrees_setting(value, name)?,
        )),
        name if name == CAMERA_ASSIGNMENT_OPTIONS3[2] => Ok(ModelSetting3::CameraZoom(
            parse_zoom_milli_setting(value, name)?,
        )),
        _ => Err(message(format!("unknown camera setting: {name}"))),
    }
}

fn parse_grid_setting_line(line: &str) -> Result<ModelSetting3, ParseError3> {
    match line {
        line if line == GRID_BARE_OPTIONS3[0] => Ok(ModelSetting3::OccupiedCellGrid),
        _ => Err(message(format!("unknown grid setting: {line}"))),
    }
}

fn parse_pixelate_setting_line(line: &str) -> Result<ModelSetting3, ParseError3> {
    if line == PIXELATE_BARE_OPTIONS3[0] {
        return Ok(ModelSetting3::PixelateSmoothing);
    }
    let (name, value) = parse_setting_assignment(line, "pixelate setting")?;
    match name {
        name if name == PIXELATE_ASSIGNMENT_OPTIONS3[0] => Ok(ModelSetting3::PixelateScale(
            parse_viewport_size_value(value, "pixelate scale")?,
        )),
        _ => Err(message(format!("unknown pixelate setting: {name}"))),
    }
}

fn parse_render_setting_line(line: &str) -> Result<ModelSetting3, ParseError3> {
    match line {
        line if line == RENDER_BARE_OPTIONS3[0] => Ok(ModelSetting3::SpriteShade),
        _ => Err(message(format!("unknown render setting: {line}"))),
    }
}

fn parse_camera_inline(rest: &str) -> Result<Vec<ModelSetting3>, ParseError3> {
    rest.split_whitespace()
        .map(parse_camera_inline_token)
        .collect()
}

fn parse_camera_inline_token(token: &str) -> Result<ModelSetting3, ParseError3> {
    match token {
        token if token == CAMERA_BARE_OPTIONS3[0] => Ok(ModelSetting3::InteractiveLook),
        token if token == CAMERA_BARE_OPTIONS3[1] => Ok(ModelSetting3::InteractiveZoom),
        _ => {
            let (name, value) = parse_inline_assignment(token, "camera option")?;
            match name {
                name if name == CAMERA_ASSIGNMENT_OPTIONS3[0] => Ok(ModelSetting3::CameraYaw(
                    parse_degrees_setting(value, name)?,
                )),
                name if name == CAMERA_ASSIGNMENT_OPTIONS3[1] => Ok(ModelSetting3::CameraPitch(
                    parse_degrees_setting(value, name)?,
                )),
                name if name == CAMERA_ASSIGNMENT_OPTIONS3[2] => Ok(ModelSetting3::CameraZoom(
                    parse_zoom_milli_setting(value, name)?,
                )),
                _ => Err(message(format!("unknown camera option: {name}"))),
            }
        }
    }
}

fn parse_grid_inline(rest: &str) -> Result<Vec<ModelSetting3>, ParseError3> {
    rest.split_whitespace()
        .map(|token| match token {
            token if token == GRID_BARE_OPTIONS3[0] => Ok(ModelSetting3::OccupiedCellGrid),
            _ => Err(message(format!("unknown grid option: {token}"))),
        })
        .collect()
}

fn parse_pixelate_inline(rest: &str) -> Result<Vec<ModelSetting3>, ParseError3> {
    rest.split_whitespace()
        .map(|token| match token {
            token if token == PIXELATE_BARE_OPTIONS3[0] => Ok(ModelSetting3::PixelateSmoothing),
            _ => {
                let (name, value) = parse_inline_assignment(token, "pixelate option")?;
                match name {
                    name if name == PIXELATE_ASSIGNMENT_OPTIONS3[0] => {
                        Ok(ModelSetting3::PixelateScale(parse_viewport_size_value(
                            value,
                            "pixelate scale",
                        )?))
                    }
                    _ => Err(message(format!("unknown pixelate option: {name}"))),
                }
            }
        })
        .collect()
}

fn parse_inline_assignment<'a>(
    token: &'a str,
    context: &str,
) -> Result<(&'a str, &'a str), ParseError3> {
    let (name, value) = token
        .split_once('=')
        .ok_or_else(|| message(format!("{context} must be name=value or a bare option")))?;
    if name.trim() != name || value.trim() != value || name.is_empty() || value.is_empty() {
        return Err(message(format!("{context} must be name=value")));
    }
    Ok((name, value))
}

fn parse_viewport_directive(
    line: &str,
    viewport: &mut ViewportSettings3,
) -> Result<(), ParseError3> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    match tokens.as_slice() {
        ["flickscreen", width, depth] => {
            parse_paged_viewport_directive(viewport, "flickscreen", width, depth, None)?
        }
        ["flickscreen", width, depth, height] => {
            parse_paged_viewport_directive(viewport, "flickscreen", width, depth, Some(height))?
        }
        ["zoomscreen", width, depth] => parse_centered_viewport_directive(
            viewport,
            "zoomscreen",
            ViewportFollow3::Snap,
            width,
            depth,
            None,
        )?,
        ["zoomscreen", width, depth, height] => parse_centered_viewport_directive(
            viewport,
            "zoomscreen",
            ViewportFollow3::Snap,
            width,
            depth,
            Some(height),
        )?,
        ["smoothscreen", width, depth] => parse_centered_viewport_directive(
            viewport,
            "smoothscreen",
            ViewportFollow3::Smooth,
            width,
            depth,
            None,
        )?,
        ["smoothscreen", width, depth, height] => parse_centered_viewport_directive(
            viewport,
            "smoothscreen",
            ViewportFollow3::Smooth,
            width,
            depth,
            Some(height),
        )?,
        ["focus", selector] => {
            viewport.focus = (*selector).to_string();
        }
        [other, ..] => {
            return Err(message(format!("unknown viewport directive: {other}")));
        }
        [] => {}
    }
    Ok(())
}

fn parse_paged_viewport_directive(
    viewport: &mut ViewportSettings3,
    directive: &str,
    width: &str,
    depth: &str,
    height: Option<&str>,
) -> Result<(), ParseError3> {
    viewport.mode = ViewportMode3::Paged;
    viewport.follow = ViewportFollow3::Snap;
    viewport.framing = Some(ViewportFraming3 {
        width: parse_viewport_size_value(width, &format!("{directive} width"))?,
        depth: parse_viewport_size_value(depth, &format!("{directive} depth"))?,
        height: match height {
            Some(value) => parse_viewport_height_value(value)?,
            None => ViewportHeight3::Full,
        },
    });
    Ok(())
}

fn parse_centered_viewport_directive(
    viewport: &mut ViewportSettings3,
    directive: &str,
    follow: ViewportFollow3,
    width: &str,
    depth: &str,
    height: Option<&str>,
) -> Result<(), ParseError3> {
    viewport.mode = ViewportMode3::Centered;
    viewport.follow = follow;
    viewport.framing = Some(ViewportFraming3 {
        width: parse_viewport_size_value(width, &format!("{directive} width"))?,
        depth: parse_viewport_size_value(depth, &format!("{directive} depth"))?,
        height: match height {
            Some(value) => parse_viewport_height_value(value)?,
            None => ViewportHeight3::Full,
        },
    });
    Ok(())
}

fn parse_viewport_height_value(value: &str) -> Result<ViewportHeight3, ParseError3> {
    if value == "full" {
        return Ok(ViewportHeight3::Full);
    }
    Ok(ViewportHeight3::Size(parse_viewport_size_value(
        value,
        "viewport height",
    )?))
}

fn parse_viewport_size_value(value: &str, name: &str) -> Result<u16, ParseError3> {
    let size = value
        .parse::<u16>()
        .map_err(|_| message(format!("{name} must be a positive integer")))?;
    if size == 0 {
        return Err(message(format!("{name} must be greater than zero")));
    }
    Ok(size)
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
    let (base, axes) = puzzle_authoring::split_object_spec(name)
        .ok_or_else(|| message("object row must be: Object[:axis...] layer"))?;
    Ok(ObjectSpec3 {
        name: base.to_string(),
        axes: axes.map(str::to_string).collect(),
        layer: layer.to_string(),
        visual: puzzle_authoring::is_display_object_token(name),
    })
}

fn parse_key_input_spec(line: &str) -> Result<InputSpec3, ParseError3> {
    let (keys, name) = line
        .split_once("->")
        .ok_or_else(|| message("keys row must be: <key...> -> <input>"))?;
    let name = name.trim();
    if name.is_empty() {
        return Err(message("keys row must name an input after ->"));
    }
    let keys = keys
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if keys.is_empty() {
        return Err(message("keys row must include at least one key before ->"));
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
    let (base, axes) = puzzle_authoring::split_object_spec(token)
        .ok_or_else(|| message("layer object must be Object[:axis...]"))?;
    reject_occurrence_label_marker_in_name(base, "object name")?;
    Ok(ObjectSpec3 {
        name: base.to_string(),
        axes: axes.map(str::to_string).collect(),
        layer: layer.to_string(),
        visual: puzzle_authoring::is_display_object_token(token),
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
    let name = name.trim();
    reject_occurrence_label_marker_in_name(name, "group name")?;
    Ok(GroupSpec3 {
        name: name.to_string(),
        selectors,
    })
}

fn reject_occurrence_label_marker_in_name(name: &str, label: &str) -> Result<(), ParseError3> {
    if name.contains('#') {
        return Err(message(format!("{label} must not contain #")));
    }
    Ok(())
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

fn parse_sprite3_shape_header(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("shape ")?;
    let name = rest.strip_suffix(" {")?;
    is_canonical_sprite_shape_name(name).then_some(name)
}

fn is_canonical_sprite_shape_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':' | '@'))
}

fn parse_sprite3_shape_block(
    lines: &[String],
    mut index: usize,
    name: &str,
) -> Result<(usize, Vec<String>), ParseError3> {
    let mut rows = Vec::new();
    while index < lines.len() {
        let line = lines[index].clone();
        if line == "}" {
            if rows.is_empty() {
                return Err(message(format!("sprite3 shape {name} requires voxel rows")));
            }
            split_level_slices(&rows)?;
            return Ok((index + 1, rows));
        }
        if line.starts_with("sprite ") || line == "colors {" || line == "voxels {" {
            return Err(message(
                "sprites3 entries must use canonical form: <name>, color row, voxel rows or shape ref",
            ));
        }
        rows.push(line);
        index += 1;
    }
    Err(message(format!("sprite3 shape {name} missing }}")))
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
    legend.insert('.', Vec::new());
    for spec in specs {
        let mut objects = Vec::new();
        if spec.selectors.len() == 1 && spec.selectors[0] == "empty" {
            if spec.ch != '.' {
                return Err(message(format!(
                    "3D levels use `.` for empty; remove `{}` = empty",
                    spec.ch
                )));
            }
            continue;
        }
        if spec.ch == '.' {
            return Err(message(
                "3D levels reserve `.` for empty; use another legend char for objects",
            ));
        }
        if legend.contains_key(&spec.ch) {
            return Err(message(format!("duplicate legend char: {}", spec.ch)));
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

fn parse_optional_program_local_frame(
    modifier: Option<&str>,
    catalog: &SelectorCatalog3,
) -> Result<Option<LocalFrame<ObjectId>>, ParseError3> {
    let Some(modifier) = modifier else {
        return Ok(None);
    };
    let tokens = modifier.split_whitespace().collect::<Vec<_>>();
    let focus_objects = default_local_frame_focus_objects(catalog)?;
    match tokens.as_slice() {
        ["local_radius", radius] => {
            let radius = parse_u16_token(radius, "local_radius")?;
            Ok(Some(LocalFrame::new(
                LocalFrameExtent::Radius(radius),
                LocalFrameExtent::Radius(radius),
                LocalFrameExtent::Radius(radius),
                focus_objects,
            )))
        }
        ["local_frame", x, y, z] => Ok(Some(LocalFrame::new(
            parse_local_frame_extent3(x)?,
            parse_local_frame_extent3(y)?,
            parse_local_frame_extent3(z)?,
            focus_objects,
        ))),
        _ => Err(message(
            "transition block header must use local_radius <n> or local_frame <x> <y> <z>",
        )),
    }
}

fn preprocess_source_lines3(source: &str) -> Vec<String> {
    let raw_lines = source
        .lines()
        .map(strip_comment)
        .map(str::trim)
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut lines = Vec::new();
    let mut block_stack = Vec::<String>::new();
    for line in raw_lines {
        if line.is_empty() {
            lines.push(String::new());
            continue;
        }
        let split_semicolons = !block_stack
            .iter()
            .any(|block| matches!(block.as_str(), "levels3" | "sprites3"));
        for piece in split_structural_line3(&line, split_semicolons) {
            update_structural_block_stack3(&piece, &mut block_stack);
            lines.push(piece);
        }
    }
    lines
}

fn update_structural_block_stack3(line: &str, stack: &mut Vec<String>) {
    if line == "}" {
        stack.pop();
        return;
    }
    if !line.ends_with('{') {
        return;
    }
    if let Some(first) = line.split_whitespace().next() {
        stack.push(first.to_string());
    }
}

fn split_structural_line3(line: &str, split_semicolons: bool) -> Vec<String> {
    let mut pieces = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut square_depth = 0usize;
    let mut paren_depth = 0usize;
    let mut inline_brace_depth = 0usize;

    for (index, ch) in line.char_indices() {
        if in_string {
            current.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
            current.push(ch);
            continue;
        }
        if inline_brace_depth > 0 {
            current.push(ch);
            if ch == '{' {
                inline_brace_depth += 1;
            } else if ch == '}' {
                inline_brace_depth = inline_brace_depth.saturating_sub(1);
            }
            continue;
        }
        match ch {
            '[' => {
                square_depth += 1;
                current.push(ch);
            }
            ']' => {
                square_depth = square_depth.saturating_sub(1);
                current.push(ch);
            }
            '(' => {
                paren_depth += 1;
                current.push(ch);
            }
            ')' => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(ch);
            }
            '{' if square_depth == 0 && paren_depth == 0 => {
                if is_inline_selector_brace3(line, index) {
                    inline_brace_depth = 1;
                    current.push(ch);
                    continue;
                }
                push_trimmed_piece3(&mut pieces, &current);
                current.clear();
                if let Some(last) = pieces.last_mut() {
                    last.push_str(" {");
                } else {
                    pieces.push("{".to_string());
                }
            }
            '}' if square_depth == 0 && paren_depth == 0 => {
                push_trimmed_piece3(&mut pieces, &current);
                current.clear();
                pieces.push("}".to_string());
            }
            ';' if split_semicolons && square_depth == 0 && paren_depth == 0 => {
                push_trimmed_piece3(&mut pieces, &current);
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    push_trimmed_piece3(&mut pieces, &current);
    pieces
}

fn push_trimmed_piece3(pieces: &mut Vec<String>, piece: &str) {
    let trimmed = piece.trim();
    if !trimmed.is_empty() {
        pieces.push(trimmed.to_string());
    }
}

fn is_inline_selector_brace3(line: &str, index: usize) -> bool {
    let before = line[..index].chars().next_back();
    let after = line[index + 1..].chars().next();
    before.is_some_and(is_selector_token_char3) && after.is_some_and(|ch| !ch.is_whitespace())
}

fn is_selector_token_char3(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '@' | ':' | '*')
}

fn collect_multiline_rule_line(
    lines: &[String],
    start: usize,
) -> Result<(String, usize), ParseError3> {
    let first = lines[start].trim();
    if !looks_like_rule_line_start(first) {
        return Ok((first.to_string(), start + 1));
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
            return Ok((first.to_string(), start + 1));
        }
        if !joined.is_empty() {
            if bracket_depth > 0 {
                joined.push_str("; ");
            } else {
                joined.push(' ');
            }
        }
        joined.push_str(line);
        bracket_depth = update_square_bracket_depth3(bracket_depth, line);
        saw_arrow |= line.contains("->");

        if index == start && bracket_depth == 0 {
            return Ok((first.to_string(), start + 1));
        }
        if index > start && bracket_depth == 0 && saw_arrow {
            return Ok((joined, index + 1));
        }
        index += 1;
    }

    Ok((first.to_string(), start + 1))
}

fn looks_like_rule_line_start(line: &str) -> bool {
    line.contains('[')
        && (line.starts_with("input ")
            || line
                .split_once(' ')
                .is_some_and(|(prefix, _)| !prefix.is_empty()))
}

fn update_square_bracket_depth3(mut depth: usize, line: &str) -> usize {
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

fn parse_local_frame_extent3(token: &str) -> Result<LocalFrameExtent, ParseError3> {
    if token == "full" {
        return Ok(LocalFrameExtent::Full);
    }
    parse_u16_token(token, "local_frame").map(LocalFrameExtent::Radius)
}

fn parse_u16_token(token: &str, context: &str) -> Result<u16, ParseError3> {
    token
        .parse::<u16>()
        .map_err(|_| message(format!("{context} value must be a non-negative integer")))
}

fn default_local_frame_focus_objects(
    catalog: &SelectorCatalog3,
) -> Result<Vec<ObjectId>, ParseError3> {
    for name in ["Player", "player"] {
        if let Some(object) = catalog.objects.iter().find(|object| object.name == name) {
            return Ok(vec![object.id]);
        }
    }
    Err(message(
        "local_frame/local_radius requires an object named Player",
    ))
}

fn parse_rule_line(
    line: &str,
    catalog: &SelectorCatalog3,
    game: &Game3,
    line_gap_limit: u16,
) -> Result<Vec<Rule3>, ParseError3> {
    if let Some(effect) = parse_camera_rule_effect_line(line)? {
        return Ok(vec![
            Rule3::once(Pattern3::new(Vec::new()), Vec::new()).with_effects(vec![effect]),
        ]);
    }

    let surface =
        puzzle_authoring::rule_statement_surface(line).map_err(|error| message(error.message()))?;
    let surface = match surface {
        puzzle_authoring::RuleStatementSurface::RuleLine(surface) => surface,
        puzzle_authoring::RuleStatementSurface::ApplicationBlock { .. } => {
            return Err(message(
                "3D rule blocks do not support nested application blocks",
            ));
        }
        puzzle_authoring::RuleStatementSurface::Call { .. } => {
            return Err(message("3D rule blocks do not support routine calls"));
        }
    };
    let (application, prefix, rest) = match surface {
        puzzle_authoring::RuleLineSurface::StandardStep(
            puzzle_authoring::StandardRuleStepSurface::Move,
        ) => return Ok(standard_move_rules3(game)),
        puzzle_authoring::RuleLineSurface::InputRewrite {
            application,
            surface,
        } => {
            let orientation = match surface.orientation {
                Some(orientation) => parse_line_orientation(orientation)?,
                None => LineOrientation3::DirectionSet(DirectionSet3::Directions),
            };
            let (lhs, rhs, effects) = parse_rewrite(surface.rewrite)?;
            let mut rules =
                lower_input_line_rewrite(catalog, orientation, lhs, rhs, line_gap_limit).map_err(
                    |error| message(format!("failed to lower input line rule: {error}")),
                )?;
            apply_rule_application(&mut rules, application)?;
            attach_rule_effects(&mut rules, &effects);
            return Ok(rules);
        }
        puzzle_authoring::RuleLineSurface::NeutralRewrite {
            application,
            rewrite,
        } => {
            let (lhs, rhs, effects) = parse_rewrite(rewrite)?;
            let mut rules = lower_line_rewrite(
                catalog,
                LineOrientation3::DirectionSet(DirectionSet3::Directions),
                lhs,
                rhs,
                line_gap_limit,
            )
            .map_err(|error| message(format!("failed to lower line rule: {error}")))?;
            apply_rule_application(&mut rules, application)?;
            attach_rule_effects(&mut rules, &effects);
            return Ok(rules);
        }
        puzzle_authoring::RuleLineSurface::OrientedRewrite {
            application,
            orientation,
            rewrite,
        } => (application, orientation, rewrite),
    };
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
        apply_rule_application(&mut rules, application)?;
        attach_rule_effects(&mut rules, &effects);
        return Ok(rules);
    }

    let orientation = parse_line_orientation(prefix)?;
    let mut rules = lower_line_rewrite(catalog, orientation, lhs, rhs, line_gap_limit)
        .map_err(|error| message(format!("failed to lower line rule: {error}")))?;
    apply_rule_application(&mut rules, application)?;
    attach_rule_effects(&mut rules, &effects);
    Ok(rules)
}

fn apply_rule_application(
    rules: &mut [Rule3],
    application: Option<puzzle_authoring::RuleApplicationSurface>,
) -> Result<(), ParseError3> {
    let Some(application) = application else {
        return Ok(());
    };
    let application = match application {
        puzzle_authoring::RuleApplicationSurface::Once => crate::RuleApplication3::Once,
        puzzle_authoring::RuleApplicationSurface::OnceAll => crate::RuleApplication3::OnceAll,
        puzzle_authoring::RuleApplicationSurface::OncePerLevel => {
            crate::RuleApplication3::OncePerLevel
        }
        puzzle_authoring::RuleApplicationSurface::Random => {
            return Err(message(
                "random rule application is not supported for puzzle3 rules",
            ));
        }
        puzzle_authoring::RuleApplicationSurface::Repeat => crate::RuleApplication3::UntilStable,
    };
    for rule in rules {
        rule.application = application;
    }
    Ok(())
}

fn standard_move_rules3(game: &Game3) -> Vec<Rule3> {
    let mut rules = Vec::new();
    let directions = Direction3::directions();
    let mut layer_objects = HashMap::<LayerId, Vec<ObjectId>>::new();
    for object in &game.objects {
        layer_objects
            .entry(object.layer_id)
            .or_default()
            .push(object.id);
    }
    for (layer, objects) in layer_objects {
        if objects.is_empty() {
            continue;
        }
        for (direction_index, direction) in directions.iter().enumerate() {
            let binding = 0;
            let mut cell = MatchCell3::new(Offset3::ZERO);
            cell.require_object_sets.push(ObjectSetMatcher3 {
                binding,
                layer,
                objects: objects.clone(),
            });
            cell.require_object_set_scratch
                .push(crate::ObjectSetScratchPattern3 {
                    binding,
                    scratch: ScratchId3(puzzle_authoring::ANONYMOUS_MOVEMENT_SCRATCH_INDEX),
                    value: Some(direction_index as i64),
                    match_value: puzzle_kernel::ScratchValueMatch::Exact,
                });
            let mut destination = MatchCell3::new(direction.offset);
            for layer_object in &objects {
                destination = destination.forbid(*layer_object);
            }
            rules.push(Rule3::repeated(
                Pattern3::new(vec![cell, destination]),
                vec![
                    WriteOp3::MoveObjectSet {
                        component: 0,
                        from_offset: Offset3::ZERO,
                        to_offset: direction.offset,
                        binding,
                    },
                    WriteOp3::RemoveObjectSetScratch {
                        component: 0,
                        offset: direction.offset,
                        binding,
                        scratch: ScratchId3(puzzle_authoring::ANONYMOUS_MOVEMENT_SCRATCH_INDEX),
                        value: None,
                        match_value: puzzle_kernel::ScratchValueMatch::Any,
                    },
                ],
            ));
        }
    }
    rules
}

fn attach_rule_effects(rules: &mut [Rule3], effects: &[RuleEffect3]) {
    if effects.is_empty() {
        return;
    }
    for rule in rules {
        rule.effects.extend(effects.iter().cloned());
    }
}

fn input_for_direction(direction: Direction3) -> InputId3 {
    match direction.name {
        "left" => InputId3(0),
        "right" => InputId3(1),
        "up" => InputId3(2),
        "down" => InputId3(3),
        "front" => InputId3(4),
        "back" => InputId3(5),
        _ => unreachable!("built-in directions are exhaustive"),
    }
}

fn lower_win_conditions(
    catalog: &SelectorCatalog3,
    lines: &[String],
    line_gap_limit: u16,
) -> Result<WinCondition3, ParseError3> {
    let conditions = lines
        .iter()
        .map(|line| parse_win_condition_line(line, catalog, line_gap_limit))
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
    line_gap_limit: u16,
) -> Result<WinCondition3, ParseError3> {
    if let Some((name, arg)) = parse_win_condition_call(line)? {
        return match name {
            "exists" | "some" => parse_some_condition(arg, catalog, line_gap_limit),
            "none" => parse_no_condition(arg, catalog, line_gap_limit),
            _ => Err(message(format!("unknown win condition function: {name}"))),
        };
    }
    if let Some(rest) = line.strip_prefix("some ") {
        return parse_some_condition(rest.trim(), catalog, line_gap_limit);
    }
    if let Some(rest) = line.strip_prefix("no ") {
        return parse_no_condition(rest.trim(), catalog, line_gap_limit);
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
    line_gap_limit: u16,
) -> Result<WinCondition3, ParseError3> {
    if rest.contains('[') {
        return pattern_conditions(rest, catalog, line_gap_limit, WinCondition3::SomePattern);
    }
    let objects = resolve_selector_objects(catalog, rest)?;
    Ok(any_object_condition(objects, WinCondition3::SomeObject))
}

fn parse_no_condition(
    rest: &str,
    catalog: &SelectorCatalog3,
    line_gap_limit: u16,
) -> Result<WinCondition3, ParseError3> {
    if rest.contains('[') {
        return pattern_conditions(rest, catalog, line_gap_limit, WinCondition3::NoPattern);
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
    line_gap_limit: u16,
    wrap: fn(Pattern3) -> WinCondition3,
) -> Result<WinCondition3, ParseError3> {
    let patterns = parse_oriented_patterns(rest, catalog, line_gap_limit)?;
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
    line_gap_limit: u16,
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
    lower_line_patterns(catalog, orientation, inner, line_gap_limit)
        .map(|rules| rules.into_iter().map(|rule| rule.pattern).collect())
        .map_err(|error| message(format!("failed to lower win pattern: {error}")))
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

fn lower_line_patterns(
    catalog: &SelectorCatalog3,
    orientation: LineOrientation3,
    inner: &str,
    line_gap_limit: u16,
) -> Result<Vec<Rule3>, String> {
    let pattern = parse_line_pattern_with_gaps(inner, catalog).map_err(parse_error_message)?;
    let mut rules = Vec::new();
    for gaps in line_gap_assignments(pattern.gap_count, line_gap_limit) {
        let rule = LineRuleTemplate3::once(
            orientation.clone(),
            pattern.materialize(&gaps).map_err(parse_error_message)?,
            Vec::new(),
        );
        rules.extend(
            lower_line_rule_template(catalog, &rule).map_err(|error| format!("{error:?}"))?,
        );
    }
    Ok(rules)
}

fn parse_error_message(error: ParseError3) -> String {
    match error {
        ParseError3::Message(message) => message,
    }
}

fn lower_line_rewrite(
    catalog: &SelectorCatalog3,
    orientation: LineOrientation3,
    lhs: &str,
    rhs: &str,
    line_gap_limit: u16,
) -> Result<Vec<Rule3>, String> {
    let before = parse_line_pattern_with_gaps(lhs, catalog).map_err(parse_error_message)?;
    let after = parse_line_pattern_with_gaps(rhs, catalog).map_err(parse_error_message)?;
    if before.gap_count != after.gap_count {
        return Err("line rewrite sides must contain the same number of ... gaps".to_string());
    }
    let mut rules = Vec::new();
    for (before, after) in expand_line_movement_scratch_sets3(&before, &after) {
        let writes = infer_line_writes_from_patterns(&before, &after);
        for gaps in line_gap_assignments(before.gap_count, line_gap_limit) {
            let rule = LineRuleTemplate3::once(
                orientation.clone(),
                before.materialize(&gaps).map_err(parse_error_message)?,
                materialize_line_writes(&writes, &gaps).map_err(parse_error_message)?,
            );
            rules.extend(
                lower_line_rule_template(catalog, &rule).map_err(|error| format!("{error:?}"))?,
            );
        }
    }
    Ok(rules)
}

fn lower_input_line_rewrite(
    catalog: &SelectorCatalog3,
    orientation: LineOrientation3,
    lhs: &str,
    rhs: &str,
    line_gap_limit: u16,
) -> Result<Vec<Rule3>, String> {
    let before = parse_line_pattern_with_gaps(lhs, catalog).map_err(parse_error_message)?;
    let after = parse_line_pattern_with_gaps(rhs, catalog).map_err(parse_error_message)?;
    if before.gap_count != after.gap_count {
        return Err("line rewrite sides must contain the same number of ... gaps".to_string());
    }
    let mut rules = Vec::new();
    for (before, after) in expand_line_movement_scratch_sets3(&before, &after) {
        let writes = infer_line_writes_from_patterns(&before, &after);
        for gaps in line_gap_assignments(before.gap_count, line_gap_limit) {
            for direction in directions_for_line_orientation(orientation.clone()) {
                let rule = LineRuleTemplate3::once(
                    LineOrientation3::Direction(direction),
                    before.materialize(&gaps).map_err(parse_error_message)?,
                    materialize_line_writes(&writes, &gaps).map_err(parse_error_message)?,
                );
                let input = input_for_direction(direction);
                let mut lowered = lower_line_rule_template(catalog, &rule)
                    .map_err(|error| format!("{error:?}"))?;
                for rule in &mut lowered {
                    rule.guards.push(crate::Guard3::InputIs(input));
                }
                rules.extend(lowered);
            }
        }
    }
    Ok(rules)
}

fn directions_for_line_orientation(orientation: LineOrientation3) -> Vec<Direction3> {
    match orientation {
        LineOrientation3::Direction(direction) => vec![direction],
        LineOrientation3::DirectionSet(set) => set.directions(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScratchSetBinding3 {
    key: String,
    values: &'static [&'static str],
}

fn expand_line_movement_scratch_sets3(
    before: &LinePatternWithGaps3,
    after: &LinePatternWithGaps3,
) -> Vec<(LinePatternWithGaps3, LinePatternWithGaps3)> {
    let mut bindings = Vec::<ScratchSetBinding3>::new();
    collect_line_movement_scratch_set_bindings3(before, &mut bindings);
    collect_line_movement_scratch_set_bindings3(after, &mut bindings);
    dedup_line_scratch_set_bindings3(&mut bindings);

    if bindings.is_empty() {
        return vec![(before.clone(), after.clone())];
    }

    let mut assignments = Vec::<HashMap<String, String>>::new();
    expand_line_scratch_set_assignments3(&bindings, 0, &mut HashMap::new(), &mut assignments);
    assignments
        .into_iter()
        .map(|assignment| {
            (
                apply_line_movement_scratch_set_assignment3(before, &assignment),
                apply_line_movement_scratch_set_assignment3(after, &assignment),
            )
        })
        .collect()
}

fn collect_line_movement_scratch_set_bindings3(
    pattern: &LinePatternWithGaps3,
    bindings: &mut Vec<ScratchSetBinding3>,
) {
    let mut selector_counts = HashMap::<String, usize>::new();
    for (cell_index, cell) in pattern.cells.iter().enumerate() {
        for selector in &cell.require {
            let ordinal = *selector_counts.get(&selector.token()).unwrap_or(&0);
            selector_counts.insert(selector.token(), ordinal + 1);
            collect_selector_scratch_set_bindings3(
                selector,
                &format!("cell:{cell_index}:require:{}:{ordinal}", selector.token()),
                bindings,
            );
        }
        for selector in &cell.forbid {
            let ordinal = *selector_counts.get(&selector.token()).unwrap_or(&0);
            selector_counts.insert(selector.token(), ordinal + 1);
            collect_selector_scratch_set_bindings3(
                selector,
                &format!("cell:{cell_index}:forbid:{}:{ordinal}", selector.token()),
                bindings,
            );
        }
    }
}

fn collect_selector_scratch_set_bindings3(
    selector: &ObjectSelector3,
    anchor: &str,
    bindings: &mut Vec<ScratchSetBinding3>,
) {
    match selector {
        ObjectSelector3::Labeled { selector, .. } => {
            collect_selector_scratch_set_bindings3(selector, anchor, bindings);
        }
        ObjectSelector3::WithScratch { selector, scratch } => {
            collect_selector_scratch_set_bindings3(selector, anchor, bindings);
            for (scratch_index, scratch) in scratch.iter().enumerate() {
                let Some(value) = scratch.value.as_deref() else {
                    continue;
                };
                let Some(values) = line_movement_scratch_set_values3(value) else {
                    continue;
                };
                bindings.push(ScratchSetBinding3 {
                    key: format!("{anchor}:scratch:{scratch_index}:{value}"),
                    values,
                });
            }
        }
        ObjectSelector3::Object(_)
        | ObjectSelector3::Group(_)
        | ObjectSelector3::Variant { .. } => {}
    }
}

fn dedup_line_scratch_set_bindings3(bindings: &mut Vec<ScratchSetBinding3>) {
    let mut deduped = Vec::with_capacity(bindings.len());
    for binding in bindings.drain(..) {
        if !deduped
            .iter()
            .any(|existing: &ScratchSetBinding3| existing.key == binding.key)
        {
            deduped.push(binding);
        }
    }
    *bindings = deduped;
}

fn expand_line_scratch_set_assignments3(
    bindings: &[ScratchSetBinding3],
    index: usize,
    current: &mut HashMap<String, String>,
    out: &mut Vec<HashMap<String, String>>,
) {
    if index == bindings.len() {
        out.push(current.clone());
        return;
    }
    let binding = &bindings[index];
    for value in binding.values {
        current.insert(binding.key.clone(), (*value).to_string());
        expand_line_scratch_set_assignments3(bindings, index + 1, current, out);
    }
    current.remove(&binding.key);
}

fn apply_line_movement_scratch_set_assignment3(
    pattern: &LinePatternWithGaps3,
    assignment: &HashMap<String, String>,
) -> LinePatternWithGaps3 {
    let mut pattern = pattern.clone();
    let mut selector_counts = HashMap::<String, usize>::new();
    for (cell_index, cell) in pattern.cells.iter_mut().enumerate() {
        for selector in &mut cell.require {
            let token = selector.token();
            let ordinal = *selector_counts.get(&token).unwrap_or(&0);
            selector_counts.insert(token.clone(), ordinal + 1);
            apply_selector_scratch_set_assignment3(
                selector,
                &format!("cell:{cell_index}:require:{token}:{ordinal}"),
                assignment,
            );
        }
        for selector in &mut cell.forbid {
            let token = selector.token();
            let ordinal = *selector_counts.get(&token).unwrap_or(&0);
            selector_counts.insert(token.clone(), ordinal + 1);
            apply_selector_scratch_set_assignment3(
                selector,
                &format!("cell:{cell_index}:forbid:{token}:{ordinal}"),
                assignment,
            );
        }
    }
    pattern
}

fn apply_selector_scratch_set_assignment3(
    selector: &mut ObjectSelector3,
    anchor: &str,
    assignment: &HashMap<String, String>,
) {
    match selector {
        ObjectSelector3::Labeled { selector, .. } => {
            apply_selector_scratch_set_assignment3(selector, anchor, assignment);
        }
        ObjectSelector3::WithScratch { selector, scratch } => {
            apply_selector_scratch_set_assignment3(selector, anchor, assignment);
            for (scratch_index, scratch) in scratch.iter_mut().enumerate() {
                let Some(value) = scratch.value.as_deref() else {
                    continue;
                };
                if line_movement_scratch_set_values3(value).is_none() {
                    continue;
                }
                let key = format!("{anchor}:scratch:{scratch_index}:{value}");
                if let Some(concrete) = assignment.get(&key) {
                    scratch.value = Some(concrete.clone());
                }
            }
        }
        ObjectSelector3::Object(_)
        | ObjectSelector3::Group(_)
        | ObjectSelector3::Variant { .. } => {}
    }
}

fn line_movement_scratch_set_values3(value: &str) -> Option<&'static [&'static str]> {
    match value {
        "horizontal" | "vertical" => puzzle_authoring::movement_scratch_set_values(value, 3),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LinePatternWithGaps3 {
    cells: Vec<LineCellWithGapStep3>,
    gap_count: u16,
}

impl LinePatternWithGaps3 {
    fn materialize(&self, gaps: &[u16]) -> Result<LinePatternTemplate3, ParseError3> {
        Ok(LinePatternTemplate3::new(
            self.cells
                .iter()
                .map(|cell| {
                    Ok(LineMatchCellTemplate3 {
                        step: cell.step.materialize(gaps)?,
                        require: cell.require.clone(),
                        forbid: cell.forbid.clone(),
                    })
                })
                .collect::<Result<Vec<_>, ParseError3>>()?,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LineCellWithGapStep3 {
    step: LineStepExpr3,
    require: Vec<ObjectSelector3>,
    forbid: Vec<ObjectSelector3>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct LineStepExpr3 {
    base: i16,
    gap_terms: Vec<u16>,
}

impl LineStepExpr3 {
    fn materialize(&self, gaps: &[u16]) -> Result<i16, ParseError3> {
        let mut step = i32::from(self.base);
        for gap_index in &self.gap_terms {
            let gap = gaps
                .get(usize::from(*gap_index))
                .ok_or_else(|| message("internal 3D gap index out of bounds"))?;
            step += i32::from(*gap);
        }
        i16::try_from(step).map_err(|_| message("3D line gap offset is too large"))
    }
}

fn parse_line_pattern_with_gaps(
    inner: &str,
    catalog: &SelectorCatalog3,
) -> Result<LinePatternWithGaps3, ParseError3> {
    let mut cells = Vec::new();
    let mut visible_step = 0_i16;
    let mut gap_count = 0_u16;
    for cell in split_line_cells(inner) {
        if cell == "..." {
            gap_count = gap_count
                .checked_add(1)
                .ok_or_else(|| message("too many 3D line gaps"))?;
            continue;
        }
        let parsed = parse_cell(cell, catalog)?;
        cells.push(LineCellWithGapStep3 {
            step: LineStepExpr3 {
                base: visible_step,
                gap_terms: (0..gap_count).collect(),
            },
            require: parsed.require,
            forbid: parsed.forbid,
        });
        visible_step = visible_step
            .checked_add(1)
            .ok_or_else(|| message("3D line pattern is too long"))?;
    }
    Ok(LinePatternWithGaps3 { cells, gap_count })
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LineWriteWithGapStep3 {
    Add {
        to: LineStepExpr3,
        object: ObjectSelector3,
    },
    Remove {
        from: LineStepExpr3,
        object: ObjectSelector3,
    },
    Move {
        from: LineStepExpr3,
        to: LineStepExpr3,
        object: ObjectSelector3,
    },
    SetScratch {
        at: LineStepExpr3,
        object: ObjectSelector3,
        scratch: SelectorScratch3,
    },
}

fn infer_line_writes_from_patterns(
    before: &LinePatternWithGaps3,
    after: &LinePatternWithGaps3,
) -> Vec<LineWriteWithGapStep3> {
    let before = positive_line_cells_from_pattern(before);
    let after = positive_line_cells_from_pattern(after);
    let after_for_scratch = after.clone();
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
            writes.push(LineWriteWithGapStep3::Remove {
                from: from.clone(),
                object: object.clone(),
            });
            continue;
        };
        used_after[index] = true;
        let to = after[index].0.clone();
        if *from != to {
            writes.push(LineWriteWithGapStep3::Move {
                from: from.clone(),
                to,
                object: object.clone(),
            });
        }
    }
    for (index, (to, object)) in after.into_iter().enumerate() {
        if !used_after[index] {
            writes.push(LineWriteWithGapStep3::Add { to, object });
        }
    }
    for (at, object) in after_for_scratch {
        for scratch in object.scratch() {
            writes.push(LineWriteWithGapStep3::SetScratch {
                at: at.clone(),
                object: object.clone(),
                scratch: scratch.clone(),
            });
        }
    }
    writes
}

fn positive_line_cells_from_pattern(
    pattern: &LinePatternWithGaps3,
) -> Vec<(LineStepExpr3, ObjectSelector3)> {
    let mut out = Vec::new();
    for cell in &pattern.cells {
        for selector in &cell.require {
            out.push((cell.step.clone(), selector.clone()));
        }
    }
    out
}

fn materialize_line_writes(
    writes: &[LineWriteWithGapStep3],
    gaps: &[u16],
) -> Result<Vec<LineWriteOpTemplate3>, ParseError3> {
    writes
        .iter()
        .map(|write| match write {
            LineWriteWithGapStep3::Add { to, object } => Ok(LineWriteOpTemplate3::Add {
                step: to.materialize(gaps)?,
                object: object.clone(),
            }),
            LineWriteWithGapStep3::Remove { from, object } => Ok(LineWriteOpTemplate3::Remove {
                step: from.materialize(gaps)?,
                object: object.clone(),
            }),
            LineWriteWithGapStep3::Move { from, to, object } => Ok(LineWriteOpTemplate3::Move {
                from_step: from.materialize(gaps)?,
                to_step: to.materialize(gaps)?,
                object: object.clone(),
            }),
            LineWriteWithGapStep3::SetScratch {
                at,
                object,
                scratch,
            } => Ok(LineWriteOpTemplate3::SetScratch {
                step: at.materialize(gaps)?,
                object: object.clone(),
                scratch: scratch.clone(),
            }),
        })
        .collect()
}

fn line_gap_assignments(gap_count: u16, line_gap_limit: u16) -> Vec<Vec<u16>> {
    if gap_count == 0 {
        return vec![Vec::new()];
    }
    let mut out = Vec::new();
    let mut current = Vec::with_capacity(usize::from(gap_count));
    collect_line_gap_assignments(gap_count, line_gap_limit, &mut current, &mut out);
    out
}

fn collect_line_gap_assignments(
    gap_count: u16,
    remaining: u16,
    current: &mut Vec<u16>,
    out: &mut Vec<Vec<u16>>,
) {
    if current.len() == usize::from(gap_count) {
        out.push(current.clone());
        return;
    }
    for gap in 0..=remaining {
        current.push(gap);
        collect_line_gap_assignments(gap_count, remaining - gap, current, out);
        current.pop();
    }
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
    let tokens = puzzle_authoring::split_cell_tokens(cell).map_err(|error| match error {
        puzzle_authoring::CellTokenError::UnmatchedCloseBrace => {
            message("scratch block has unmatched }")
        }
        puzzle_authoring::CellTokenError::MissingCloseBrace => message("scratch block missing }"),
    })?;
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
        } else if puzzle_authoring::scratch_sugar_kind(&tokens[index]).is_some() {
            let selector = tokens
                .get(index + 1)
                .ok_or_else(|| message("scratch sugar must be followed by a selector"))?;
            if selector == "no" || puzzle_authoring::scratch_sugar_kind(selector).is_some() {
                return Err(message("scratch sugar must be followed by a selector"));
            }
            let selector = parse_selector(selector, &catalog.families, &catalog.groups)?;
            require.push(ObjectSelector3::with_scratch(
                selector,
                vec![anonymous_selector_scratch(&tokens[index], false)],
            ));
            index += 2;
        } else {
            require.push(parse_selector(
                &tokens[index],
                &catalog.families,
                &catalog.groups,
            )?);
            index += 1;
        }
    }
    Ok(ParsedCell3 { require, forbid })
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
    let (selector, scratch) = split_selector_scratch3(token)?;
    let (selector, occurrence_label) = split_selector_occurrence_label3(selector)?;
    let parts = selector.split(':').collect::<Vec<_>>();
    let parsed = if parts.len() > 1 {
        ObjectSelector3::variant(
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
        )
    } else {
        if families.iter().any(|family| family.name == selector) {
            return Err(message(format!(
                "variant selector must use explicit tags: {selector}"
            )));
        }
        if groups.iter().any(|group| group.name == selector) {
            ObjectSelector3::group(selector)
        } else {
            ObjectSelector3::object(selector)
        }
    };
    let parsed = match occurrence_label {
        Some(label) => ObjectSelector3::labeled(format!("{selector}#{label}"), parsed),
        None => parsed,
    };
    Ok(ObjectSelector3::with_scratch(parsed, scratch))
}

fn split_selector_scratch3(selector: &str) -> Result<(&str, Vec<SelectorScratch3>), ParseError3> {
    let Some(open_index) = selector.find('{') else {
        return Ok((selector, Vec::new()));
    };
    let base = &selector[..open_index];
    let attrs = selector[open_index + 1..]
        .strip_suffix('}')
        .ok_or_else(|| message("scratch selector must end with }"))?;
    if base.is_empty() {
        return Err(message("scratch selector must attach to an object"));
    }
    Ok((base, parse_selector_scratch3(attrs)?))
}

fn parse_selector_scratch3(attrs: &str) -> Result<Vec<SelectorScratch3>, ParseError3> {
    let mut parsed = Vec::new();
    let tokens = attrs.split_whitespace().collect::<Vec<_>>();
    let mut index = 0;
    while index < tokens.len() {
        let (negated, spec) = if tokens[index] == "no" {
            let spec = tokens
                .get(index + 1)
                .ok_or_else(|| message("no must be followed by a scratch"))?;
            index += 2;
            (true, *spec)
        } else {
            let spec = tokens[index];
            index += 1;
            (false, spec)
        };
        if puzzle_authoring::scratch_sugar_kind(spec).is_some() {
            parsed.push(anonymous_selector_scratch(spec, negated));
            continue;
        }
        let (name, value) = spec
            .split_once('=')
            .map_or((spec, None), |(name, value)| (name, Some(value)));
        if !puzzle_authoring::is_identifier(name) {
            return Err(message("scratch name must start with an identifier"));
        }
        parsed.push(SelectorScratch3 {
            name: name.to_string(),
            value: value.map(str::to_string),
            negated,
        });
    }
    Ok(parsed)
}

fn anonymous_selector_scratch(value: &str, negated: bool) -> SelectorScratch3 {
    SelectorScratch3 {
        name: String::new(),
        value: Some(value.to_string()),
        negated,
    }
}

fn split_selector_occurrence_label3(selector: &str) -> Result<(&str, Option<String>), ParseError3> {
    let Some((base, label)) = selector.split_once('#') else {
        return Ok((selector, None));
    };
    if base.is_empty() || label.is_empty() || label.contains('#') {
        return Err(message("selector occurrence label must be: selector#label"));
    }
    if !label
        .chars()
        .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        return Err(message(
            "selector occurrence label may only contain letters, numbers, and _",
        ));
    }
    Ok((base, Some(label.to_string())))
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
        InputDef3::directional(InputId3(4), "front", Direction3::FORWARD),
        InputDef3::directional(InputId3(5), "back", Direction3::BACKWARD),
    ]
}

fn canonical_input_name3(name: &str) -> &str {
    puzzle_authoring::canonical_3d_movement_direction_name(name)
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
        let canonical_name = canonical_input_name3(&spec.name);
        if inputs
            .iter()
            .any(|input: &InputDef3| input.name == canonical_name)
        {
            return Err(message(format!("duplicate input: {}", spec.name)));
        }
        let input =
            if let Some(default) = defaults.iter().find(|input| input.name == canonical_name) {
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
