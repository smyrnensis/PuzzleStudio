use std::collections::HashMap;

use puzzle_core::{
    CompiledGame, ConditionValueKind, ExecutableProgram, GridCompiledGame, GridConditionValueKind,
    GridRuleStep, try_project_grid_compiled_game, try_project_grid_condition_value,
    try_project_grid_program,
};
use puzzle_kernel::SpatialVector;

use crate::{
    CanonicalGoalCondition, CanonicalQueryExpr, CanonicalSolverStrategy, DiagnosticReport,
    GoalCondition, QueryExpr, SolverStrategy,
};

pub(crate) fn validate_visuals(visuals: &crate::VisualsDef) -> Result<(), DiagnosticReport> {
    const EPSILON: f64 = 1e-9;
    for visual in &visuals.entries {
        if visual.frames.iter().any(|frame| frame.planes.len() > 1) {
            return Err(DiagnosticReport::error(format!(
                "2D renderer cannot materialize multi-plane visual `{}`",
                visual.name
            )));
        }
        for transform in &visual.transforms {
            match transform {
                crate::VisualTransform::Translate { value, .. } if value[2].abs() > EPSILON => {
                    return Err(DiagnosticReport::error(format!(
                        "2D renderer cannot materialize out-of-plane translation on visual `{}`",
                        visual.name
                    )));
                }
                crate::VisualTransform::Rotate { axis, .. }
                    if axis[0].abs() > EPSILON
                        || axis[1].abs() > EPSILON
                        || (axis[2].abs() - 1.0).abs() > EPSILON =>
                {
                    return Err(DiagnosticReport::error(format!(
                        "2D renderer cannot materialize rotation outside the visual plane on `{}`",
                        visual.name
                    )));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub(crate) fn game(source: &GridCompiledGame<3>) -> Result<CompiledGame, DiagnosticReport> {
    try_project_grid_compiled_game(source, project_vector)
}

pub(crate) fn executable(
    source: &[GridRuleStep<3>],
) -> Result<ExecutableProgram, DiagnosticReport> {
    Ok(ExecutableProgram::new(try_project_grid_program(
        source,
        project_vector,
    )?))
}

pub(crate) fn query(value: &CanonicalQueryExpr) -> Result<QueryExpr, DiagnosticReport> {
    value.try_map_value(&mut condition_value)
}

pub(crate) fn queries(
    values: HashMap<String, CanonicalQueryExpr>,
) -> Result<HashMap<String, QueryExpr>, DiagnosticReport> {
    values
        .into_iter()
        .map(|(name, value)| query(&value).map(|value| (name, value)))
        .collect()
}

pub(crate) fn goal(value: &CanonicalGoalCondition) -> Result<GoalCondition, DiagnosticReport> {
    value.try_map_value(&mut condition_value)
}

pub(crate) fn goals(
    values: HashMap<String, CanonicalGoalCondition>,
) -> Result<HashMap<String, GoalCondition>, DiagnosticReport> {
    values
        .into_iter()
        .map(|(name, value)| goal(&value).map(|value| (name, value)))
        .collect()
}

pub(crate) fn solver(value: &CanonicalSolverStrategy) -> Result<SolverStrategy, DiagnosticReport> {
    value.try_map_query(&mut query)
}

fn condition_value(
    value: &GridConditionValueKind<3>,
) -> Result<ConditionValueKind, DiagnosticReport> {
    try_project_grid_condition_value(value, project_vector)
}

fn project_vector(value: SpatialVector<3>) -> Result<SpatialVector<2>, DiagnosticReport> {
    let [x, y, z] = value.axes();
    if z != 0 {
        return Err(DiagnosticReport::error(
            "2D materialization received a non-zero canonical z offset".to_string(),
        ));
    }
    Ok(SpatialVector::new([x, y]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{VisualDef, VisualFit, VisualKind, VisualSpace, VisualTransform, VisualsDef};

    fn visuals_with(transform: VisualTransform) -> VisualsDef {
        VisualsDef {
            entries: vec![VisualDef {
                name: "test".to_string(),
                source_line: None,
                source_line_number: None,
                kind: VisualKind::Solid("#fff".to_string()),
                frames: Vec::new(),
                transforms: vec![transform],
                fit: VisualFit::default(),
                sampling: None,
                animation_duration_ms: None,
                pixels_per_cell: None,
            }],
            ..VisualsDef::default()
        }
    }

    #[test]
    fn planar_projection_accepts_the_2d_subspace_of_shared_visual_transforms() {
        validate_visuals(&visuals_with(VisualTransform::Translate {
            value: [1.0, -0.5, 0.0],
            space: VisualSpace::Local,
        }))
        .unwrap();
    }

    #[test]
    fn planar_projection_rejects_transform_that_leaves_the_2d_subspace() {
        let error = validate_visuals(&visuals_with(VisualTransform::Rotate {
            degrees: 45.0,
            axis: [1.0, 0.0, 0.0],
            space: VisualSpace::World,
        }))
        .unwrap_err();
        assert!(error.to_string().contains("outside the visual plane"));
    }
}
