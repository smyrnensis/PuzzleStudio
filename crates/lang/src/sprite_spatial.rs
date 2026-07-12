use crate::{SpriteSpace3, SpriteSpatialOp3};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteAffine3 {
    pub matrix: [[f64; 4]; 4],
}

impl Default for SpriteAffine3 {
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

impl SpriteAffine3 {
    pub fn transform_point(self, p: [f64; 3]) -> [f64; 3] {
        let m = self.matrix;
        [
            m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2] + m[0][3],
            m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2] + m[1][3],
            m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2] + m[2][3],
        ]
    }
}

pub fn evaluate_sprite_spatial_ops3(ops: &[SpriteSpatialOp3]) -> SpriteAffine3 {
    let mut result = SpriteAffine3::default().matrix;
    for op in ops {
        let (space, op_matrix) = match *op {
            SpriteSpatialOp3::Translate { space, value } => (space, translation(value)),
            SpriteSpatialOp3::Rotate {
                space,
                axis,
                degrees,
            } => (space, rotation(axis, degrees)),
        };
        result = match space {
            SpriteSpace3::World => multiply(op_matrix, result),
            SpriteSpace3::Local => multiply(result, op_matrix),
        };
    }
    SpriteAffine3 { matrix: result }
}

fn translation([x, y, z]: [f64; 3]) -> [[f64; 4]; 4] {
    let mut m = SpriteAffine3::default().matrix;
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
        let p = evaluate_sprite_spatial_ops3(&[
            SpriteSpatialOp3::Translate {
                space: SpriteSpace3::World,
                value: [1.0, 0.0, 0.0],
            },
            SpriteSpatialOp3::Rotate {
                space: SpriteSpace3::World,
                axis: [0.0, 0.0, 1.0],
                degrees: 90.0,
            },
        ]);
        close(p.transform_point([0.0, 0.0, 0.0]), [0.0, 1.0, 0.0]);
    }
    #[test]
    fn local_rotation_uses_translated_local_origin() {
        let p = evaluate_sprite_spatial_ops3(&[
            SpriteSpatialOp3::Translate {
                space: SpriteSpace3::World,
                value: [1.0, 0.0, 0.0],
            },
            SpriteSpatialOp3::Rotate {
                space: SpriteSpace3::Local,
                axis: [0.0, 0.0, 1.0],
                degrees: 90.0,
            },
        ]);
        close(p.transform_point([0.0, 0.0, 0.0]), [1.0, 0.0, 0.0]);
        close(p.transform_point([1.0, 0.0, 0.0]), [1.0, 1.0, 0.0]);
    }
}
