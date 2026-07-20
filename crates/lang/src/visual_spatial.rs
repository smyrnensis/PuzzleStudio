use crate::{VisualSpace, VisualTransform};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpatialVisualAffine {
    pub matrix: [[f64; 4]; 4],
}

impl Default for SpatialVisualAffine {
    fn default() -> Self {
        Self {
            matrix: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }
}

impl SpatialVisualAffine {
    pub fn transform_point(self, p: [f64; 3]) -> [f64; 3] {
        let m = self.matrix;
        [
            m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2] + m[0][3],
            m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2] + m[1][3],
            m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2] + m[2][3],
        ]
    }
}

pub fn evaluate_spatial_visual_transforms(transforms: &[VisualTransform]) -> SpatialVisualAffine {
    let mut result = SpatialVisualAffine::default().matrix;
    for transform in transforms {
        let (space, op_matrix) = match *transform {
            VisualTransform::Translate { space, value } => (space, translation(value)),
            VisualTransform::Rotate {
                space,
                axis,
                degrees,
            } => (space, rotation(axis, degrees)),
            VisualTransform::Flip { enabled } => {
                if !enabled {
                    continue;
                }
                (VisualSpace::Local, reflection_x())
            }
        };
        result = match space {
            VisualSpace::World => multiply(op_matrix, result),
            VisualSpace::Local => multiply(result, op_matrix),
        };
    }
    SpatialVisualAffine { matrix: result }
}
fn reflection_x() -> [[f64; 4]; 4] {
    let mut m = SpatialVisualAffine::default().matrix;
    m[0][0] = -1.0;
    m
}

fn translation([x, y, z]: [f64; 3]) -> [[f64; 4]; 4] {
    let mut m = SpatialVisualAffine::default().matrix;
    m[0][3] = x;
    m[1][3] = y;
    m[2][3] = z;
    m
}
fn rotation([x, y, z]: [f64; 3], degrees: f64) -> [[f64; 4]; 4] {
    let r = degrees.to_radians();
    let c = r.cos();
    let s = r.sin();
    let t = 1.0 - c;
    [
        [t * x * x + c, t * x * y - s * z, t * x * z + s * y, 0.0],
        [t * x * y + s * z, t * y * y + c, t * y * z - s * x, 0.0],
        [t * x * z - s * y, t * y * z + s * x, t * z * z + c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}
fn multiply(a: [[f64; 4]; 4], b: [[f64; 4]; 4]) -> [[f64; 4]; 4] {
    let mut o = [[0.0; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            o[r][c] = (0..4).map(|i| a[r][i] * b[i][c]).sum();
        }
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    fn close(a: [f64; 3], e: [f64; 3]) {
        for i in 0..3 {
            assert!((a[i] - e[i]).abs() < 1e-9, "{a:?} != {e:?}");
        }
    }
    #[test]
    fn world_rotation_rotates_earlier_translation() {
        let p = evaluate_spatial_visual_transforms(&[
            VisualTransform::Translate {
                space: VisualSpace::World,
                value: [1.0, 0.0, 0.0],
            },
            VisualTransform::Rotate {
                space: VisualSpace::World,
                axis: [0.0, 0.0, 1.0],
                degrees: 90.0,
            },
        ]);
        close(p.transform_point([0.0, 0.0, 0.0]), [0.0, 1.0, 0.0]);
    }
    #[test]
    fn local_rotation_uses_translated_local_origin() {
        let p = evaluate_spatial_visual_transforms(&[
            VisualTransform::Translate {
                space: VisualSpace::World,
                value: [1.0, 0.0, 0.0],
            },
            VisualTransform::Rotate {
                space: VisualSpace::Local,
                axis: [0.0, 0.0, 1.0],
                degrees: 90.0,
            },
        ]);
        close(p.transform_point([0.0, 0.0, 0.0]), [1.0, 0.0, 0.0]);
        close(p.transform_point([1.0, 0.0, 0.0]), [1.0, 1.0, 0.0]);
    }
}
