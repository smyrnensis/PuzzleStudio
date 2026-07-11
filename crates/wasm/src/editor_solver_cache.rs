#[derive(Debug, Clone, Copy)]
pub struct EditorSolverCachePolicy {
    pub version: u32,
    pub max_artifacts: usize,
    pub max_estimated_bytes: usize,
    pub idle_ttl_ms: u64,
}

/// The Editor solver cache policy is intentionally centralized here so its
/// initial tuning values do not become duplicated lifecycle behavior in JS.
pub const DEFAULT_EDITOR_SOLVER_CACHE_POLICY: EditorSolverCachePolicy = EditorSolverCachePolicy {
    version: 1,
    max_artifacts: 8,
    max_estimated_bytes: 64 * 1024 * 1024,
    idle_ttl_ms: 15 * 60 * 1000,
};

pub fn default_policy_json() -> String {
    let policy = DEFAULT_EDITOR_SOLVER_CACHE_POLICY;
    format!(
        "{{\"version\":{},\"maxArtifacts\":{},\"maxEstimatedBytes\":{},\"idleTtlMs\":{}}}",
        policy.version, policy.max_artifacts, policy.max_estimated_bytes, policy.idle_ttl_ms
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_keeps_the_tunable_limits_in_one_owner() {
        assert_eq!(DEFAULT_EDITOR_SOLVER_CACHE_POLICY.max_artifacts, 8);
        assert_eq!(
            DEFAULT_EDITOR_SOLVER_CACHE_POLICY.max_estimated_bytes,
            64 * 1024 * 1024
        );
        assert_eq!(
            DEFAULT_EDITOR_SOLVER_CACHE_POLICY.idle_ttl_ms,
            15 * 60 * 1000
        );
    }
}
