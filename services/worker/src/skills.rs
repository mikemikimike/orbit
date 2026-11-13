//! Materialize organization skills into an execution workspace.
//!
//! Pi discovers project skills under `.pi/skills`; the same files are also written to
//! `.opencode/skills` for OpenCode-style workspaces. Files are validated by the API
//! (relative paths, string content) and re-checked here.

use std::path::Path;

use orbit_domain::AgentSpec;
use orbit_store::Skill;

pub async fn materialize(
    spec: &AgentSpec,
    execution_skill_slugs: &[String],
    profile_skill_slugs: &[String],
    skills: &[Skill],
    workspace: &Path,
) -> Vec<String> {
    let mut slugs: Vec<String> = spec.skills.clone();
    slugs.extend(execution_skill_slugs.iter().cloned());
    slugs.extend(profile_skill_slugs.iter().cloned());
    slugs.sort();
    slugs.dedup();
    if slugs.is_empty() {
        return Vec::new();
    }
    let skills: Vec<&Skill> = skills
        .iter()
        .filter(|skill| slugs.contains(&skill.slug))
        .collect();
    let mut materialized = Vec::new();
    for skill in skills {
        for base in [".pi/skills", ".opencode/skills"] {
            let dir = workspace.join(base).join(&skill.slug);
            if let Err(error) = tokio::fs::create_dir_all(&dir).await {
                tracing::warn!(%error, skill = %skill.slug, "could not create skill dir");
                continue;
            }
            let _ = tokio::fs::write(dir.join("SKILL.md"), &skill.content).await;
            if let Some(files) = skill.files.as_object() {
                for (path, content) in files {
                    if path.starts_with('/') || path.contains("..") || path.contains('\\') {
                        continue;
                    }
                    if let Some(content) = content.as_str() {
                        let target = dir.join(path);
                        if let Some(parent) = target.parent() {
                            let _ = tokio::fs::create_dir_all(parent).await;
                        }
                        let _ = tokio::fs::write(&target, content).await;
                        // Make scripts executable.
                        let is_script = path.starts_with("scripts/")
                            || path.contains("/scripts/");
                        if is_script {
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                if let Ok(meta) = tokio::fs::metadata(&target).await {
                                    let mut perms = meta.permissions();
                                    perms.set_mode(0o755);
                                    let _ = tokio::fs::set_permissions(&target, perms).await;
                                }
                            }
                            tracing::info!(skill = %skill.slug, path = %path, "materialized executable script");
                        }
                    }
                }
            }
        }
        materialized.push(skill.slug.clone());
    }
    materialized
}
