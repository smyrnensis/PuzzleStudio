use std::collections::HashMap;

use puzzle_core::GridInput;
use puzzle_kernel::SpatialVector;

use crate::{ArrowKey, Controls, DiagnosticReport, DirectionalInput, InputId, ModelDimension};

pub(crate) fn materialize_inputs<const D: usize>(
    dimension: ModelDimension,
    controls: &Controls,
    input_labels: &HashMap<InputId, String>,
) -> Result<Vec<GridInput<D>>, DiagnosticReport> {
    let expected_axes = match dimension {
        ModelDimension::Two => 2,
        ModelDimension::Three => 3,
    };
    if D != expected_axes {
        return Err(DiagnosticReport::error(format!(
            "input materialization dimension mismatch: model has {expected_axes} axes, target has {D}"
        )));
    }

    let mut keys = HashMap::<_, Vec<String>>::new();
    for (key, input) in &controls.keys {
        keys.entry(*input)
            .or_default()
            .push(char::from(*key).to_string());
    }
    for (key, input) in &controls.arrows {
        let name = match key {
            ArrowKey::Up => "ArrowUp",
            ArrowKey::Down => "ArrowDown",
            ArrowKey::Left => "ArrowLeft",
            ArrowKey::Right => "ArrowRight",
        };
        keys.entry(*input).or_default().push(name.to_string());
    }
    for (key, input) in &controls.named {
        keys.entry(*input).or_default().push(key.clone());
    }

    let domain = SpatialDomain::new(dimension);
    let mut inputs = input_labels
        .iter()
        .map(|(id, name)| {
            let direction = domain.direction_vector(name).map(|vector| {
                let source = vector.axes();
                let mut axes = [0; D];
                axes.copy_from_slice(&source[..D]);
                SpatialVector::new(axes)
            });
            GridInput {
                id: *id,
                name: name.clone(),
                direction,
                keys: keys.remove(id).unwrap_or_default(),
            }
        })
        .collect::<Vec<_>>();
    inputs.sort_by_key(|input| input.id.0);
    Ok(inputs)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SpatialFrame {
    axes: [SpatialVector<3>; 3],
}

impl SpatialFrame {
    pub(crate) fn axis(self, index: usize) -> SpatialVector<3> {
        self.axes[index]
    }

    pub(crate) fn project_xy(self, x: i16, y: i16) -> SpatialVector<3> {
        add(scale(self.axes[0], x), scale(self.axes[1], y))
    }

    pub(crate) fn is_canonical_chiral(self) -> bool {
        cross(self.axes[0], self.axes[1]) == self.axes[2]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OrientationEnvironment {
    pub(crate) input: InputId,
    pub(crate) primary_name: &'static str,
    pub(crate) frame: SpatialFrame,
    dimension: ModelDimension,
}

impl OrientationEnvironment {
    pub(crate) fn dimension(self) -> ModelDimension {
        self.dimension
    }

    pub(crate) fn expand_selector(
        self,
        selector: &str,
        input: InputId,
    ) -> Result<Vec<Self>, DiagnosticReport> {
        SpatialDomain::new(self.dimension).expand_frame_selector(selector, input)
    }

    pub(crate) fn relative_vector(
        self,
        direction: puzzle_authoring::RelativeDirection,
    ) -> SpatialVector<3> {
        match direction {
            puzzle_authoring::RelativeDirection::Forward => self.frame.axis(0),
            puzzle_authoring::RelativeDirection::Backward => scale(self.frame.axis(0), -1),
            puzzle_authoring::RelativeDirection::Left => scale(self.frame.axis(1), -1),
            puzzle_authoring::RelativeDirection::Right => self.frame.axis(1),
        }
    }

    pub(crate) fn direction_name(self, vector: SpatialVector<3>) -> Option<&'static str> {
        SpatialDomain::new(self.dimension).direction_name(vector)
    }

    pub(crate) fn direction_value(self, vector: SpatialVector<3>) -> Option<i64> {
        SpatialDomain::new(self.dimension)
            .direction_names()
            .into_iter()
            .position(|name| {
                SpatialDomain::new(self.dimension)
                    .direction_vector(name)
                    .is_some_and(|candidate| candidate == vector)
            })
            .map(|index| index as i64)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SpatialDomain {
    dimension: ModelDimension,
}

impl SpatialDomain {
    pub(crate) const fn new(dimension: ModelDimension) -> Self {
        Self { dimension }
    }

    pub(crate) fn neutral(self) -> OrientationEnvironment {
        OrientationEnvironment {
            input: InputId(0),
            primary_name: "right",
            frame: self.default_frame(),
            dimension: self.dimension,
        }
    }

    pub(crate) fn expand_directional_input(
        self,
        binding: &DirectionalInput,
    ) -> Result<Vec<OrientationEnvironment>, DiagnosticReport> {
        self.expand_primary(&binding.direction, binding.input)
    }

    pub(crate) fn expand_frame_selector(
        self,
        selector: &str,
        input: InputId,
    ) -> Result<Vec<OrientationEnvironment>, DiagnosticReport> {
        let selector = selector.trim();
        let inner = selector
            .strip_prefix('(')
            .and_then(|value| value.strip_suffix(')'))
            .unwrap_or(selector);
        let slots = inner.split(',').map(str::trim).collect::<Vec<_>>();
        if slots.iter().any(|slot| slot.is_empty()) {
            return Err(DiagnosticReport::error(format!(
                "invalid frame orientation: {selector}"
            )));
        }
        match slots.as_slice() {
            [primary] => self.expand_primary(primary, input),
            [primary, secondary] => {
                let frame = self.frame_from_names(primary, secondary, None)?;
                Ok(vec![OrientationEnvironment {
                    input,
                    primary_name: self.canonical_direction_name(primary)?,
                    frame,
                    dimension: self.dimension,
                }])
            }
            [primary, secondary, depth] if self.dimension == ModelDimension::Three => {
                let frame = self.frame_from_names(primary, secondary, Some(depth))?;
                Ok(vec![OrientationEnvironment {
                    input,
                    primary_name: self.canonical_direction_name(primary)?,
                    frame,
                    dimension: self.dimension,
                }])
            }
            _ => Err(DiagnosticReport::error(format!(
                "frame orientation has the wrong number of axes for dimension {}: {selector}",
                match self.dimension {
                    ModelDimension::Two => 2,
                    ModelDimension::Three => 3,
                }
            ))),
        }
    }

    pub(crate) fn direction_name(self, vector: SpatialVector<3>) -> Option<&'static str> {
        self.direction_names().into_iter().find(|name| {
            self.direction_vector(name)
                .is_some_and(|value| value == vector)
        })
    }

    fn expand_primary(
        self,
        primary: &str,
        input: InputId,
    ) -> Result<Vec<OrientationEnvironment>, DiagnosticReport> {
        let primary_vector = self
            .direction_vector(primary)
            .ok_or_else(|| DiagnosticReport::error(format!("unknown direction name: {primary}")))?;
        let secondary_names = match self.dimension {
            ModelDimension::Two => self
                .direction_names()
                .into_iter()
                .filter(|name| {
                    self.direction_vector(name)
                        .is_some_and(|value| dot(primary_vector, value) == 0)
                        && self.frame_from_names(primary, name, None).is_ok()
                })
                .take(1)
                .collect::<Vec<_>>(),
            ModelDimension::Three => self
                .direction_names()
                .into_iter()
                .filter(|name| {
                    self.direction_vector(name)
                        .is_some_and(|value| dot(primary_vector, value) == 0)
                })
                .collect::<Vec<_>>(),
        };
        let primary_name = self.canonical_direction_name(primary)?;
        secondary_names
            .into_iter()
            .map(|secondary| {
                self.frame_from_names(primary, secondary, None)
                    .map(|frame| OrientationEnvironment {
                        input,
                        primary_name,
                        frame,
                        dimension: self.dimension,
                    })
            })
            .collect()
    }

    pub(crate) fn frame_from_names(
        self,
        primary: &str,
        secondary: &str,
        depth: Option<&str>,
    ) -> Result<SpatialFrame, DiagnosticReport> {
        let primary_vector = self.required_direction(primary)?;
        let secondary_vector = self.required_direction(secondary)?;
        if dot(primary_vector, secondary_vector) != 0 {
            return Err(DiagnosticReport::error(format!(
                "frame axes must be orthogonal: {primary}, {secondary}"
            )));
        }
        match self.dimension {
            ModelDimension::Two => {
                let expected =
                    SpatialVector::new([-primary_vector.axes()[1], primary_vector.axes()[0], 0]);
                if secondary_vector != expected {
                    return Err(DiagnosticReport::error(format!(
                        "2D frame must use canonical chirality: {primary}, {secondary}"
                    )));
                }
                Ok(SpatialFrame {
                    axes: [
                        primary_vector,
                        secondary_vector,
                        SpatialVector::new([0, 0, 1]),
                    ],
                })
            }
            ModelDimension::Three => {
                let canonical_depth = cross(primary_vector, secondary_vector);
                let depth_vector = match depth {
                    Some(name) => self.required_direction(name)?,
                    None => canonical_depth,
                };
                if dot(primary_vector, depth_vector) != 0
                    || dot(secondary_vector, depth_vector) != 0
                {
                    return Err(DiagnosticReport::error(format!(
                        "frame axes must be orthogonal: {primary}, {secondary}, {}",
                        depth.unwrap_or("*")
                    )));
                }
                Ok(SpatialFrame {
                    axes: [primary_vector, secondary_vector, depth_vector],
                })
            }
        }
    }

    fn required_direction(self, name: &str) -> Result<SpatialVector<3>, DiagnosticReport> {
        self.direction_vector(name)
            .ok_or_else(|| DiagnosticReport::error(format!("unknown direction name: {name}")))
    }

    fn canonical_direction_name(self, name: &str) -> Result<&'static str, DiagnosticReport> {
        self.direction_names()
            .into_iter()
            .find(|candidate| *candidate == name)
            .ok_or_else(|| DiagnosticReport::error(format!("unknown direction name: {name}")))
    }

    pub(crate) fn direction_names(self) -> Vec<&'static str> {
        match self.dimension {
            ModelDimension::Two => vec!["up", "down", "left", "right"],
            ModelDimension::Three => vec!["up", "down", "left", "right", "front", "back"],
        }
    }

    pub(crate) fn direction_vector(self, name: &str) -> Option<SpatialVector<3>> {
        match (self.dimension, name) {
            (ModelDimension::Two, "up") => Some(SpatialVector::new([0, -1, 0])),
            (ModelDimension::Two, "down") => Some(SpatialVector::new([0, 1, 0])),
            (ModelDimension::Two, "left") => Some(SpatialVector::new([-1, 0, 0])),
            (ModelDimension::Two, "right") => Some(SpatialVector::new([1, 0, 0])),
            (ModelDimension::Three, "up") => Some(SpatialVector::new([0, 0, 1])),
            (ModelDimension::Three, "down") => Some(SpatialVector::new([0, 0, -1])),
            (ModelDimension::Three, "left") => Some(SpatialVector::new([-1, 0, 0])),
            (ModelDimension::Three, "right") => Some(SpatialVector::new([1, 0, 0])),
            (ModelDimension::Three, "front") => Some(SpatialVector::new([0, 1, 0])),
            (ModelDimension::Three, "back") => Some(SpatialVector::new([0, -1, 0])),
            _ => None,
        }
    }

    fn default_frame(self) -> SpatialFrame {
        match self.dimension {
            ModelDimension::Two => SpatialFrame {
                axes: [
                    SpatialVector::new([1, 0, 0]),
                    SpatialVector::new([0, 1, 0]),
                    SpatialVector::new([0, 0, 1]),
                ],
            },
            ModelDimension::Three => SpatialFrame {
                axes: [
                    SpatialVector::new([1, 0, 0]),
                    SpatialVector::new([0, -1, 0]),
                    SpatialVector::new([0, 0, -1]),
                ],
            },
        }
    }
}

fn scale(vector: SpatialVector<3>, amount: i16) -> SpatialVector<3> {
    SpatialVector::new(vector.axes().map(|value| value * amount))
}

fn add(left: SpatialVector<3>, right: SpatialVector<3>) -> SpatialVector<3> {
    SpatialVector::new(std::array::from_fn(|index| {
        left.axes()[index] + right.axes()[index]
    }))
}

fn dot(left: SpatialVector<3>, right: SpatialVector<3>) -> i16 {
    (0..3)
        .map(|index| left.axes()[index] * right.axes()[index])
        .sum()
}

fn cross(left: SpatialVector<3>, right: SpatialVector<3>) -> SpatialVector<3> {
    let [lx, ly, lz] = left.axes();
    let [rx, ry, rz] = right.axes();
    SpatialVector::new([
        (ly * rz) - (lz * ry),
        (lz * rx) - (lx * rz),
        (lx * ry) - (ly * rx),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_dimensional_direction_has_one_canonical_frame() {
        let frames = SpatialDomain::new(ModelDimension::Two)
            .expand_frame_selector("up", InputId(1))
            .unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].frame.axis(0), SpatialVector::new([0, -1, 0]));
        assert_eq!(frames[0].frame.axis(1), SpatialVector::new([1, 0, 0]));
    }

    #[test]
    fn three_dimensional_direction_is_primary_with_wildcard_secondary() {
        let frames = SpatialDomain::new(ModelDimension::Three)
            .expand_frame_selector("up", InputId(1))
            .unwrap();
        assert_eq!(frames.len(), 4);
        assert!(
            frames
                .iter()
                .all(|env| { env.frame.axis(0) == SpatialVector::new([0, 0, 1]) })
        );
    }

    #[test]
    fn comma_frame_uses_canonical_completion() {
        let frames = SpatialDomain::new(ModelDimension::Three)
            .expand_frame_selector("right, up", InputId(1))
            .unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].frame.axis(2), SpatialVector::new([0, -1, 0]));
    }
}
