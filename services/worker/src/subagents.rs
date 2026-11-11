//! Materialize OpenCode-native subagents for a native Agent Team execution.
//!
//! The primary agent runs in one harness process; the remaining team agents become
//! project-local OpenCode subagents under `.opencode/agents`.

use std::path::Path;

use serde_json::Value;

fn slug(value: &str) -> String {
    let mut out = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if matches!(character, '-' | '_' | ' ') && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// Primary-agent system instruction describing available native subagents.
pub fn system_instruction(input: &Value) -> Option<String> {
    let team = input.get("team")?;
    if team.get("mode").and_then(Value::as_str) != Some("native") {
        return None;
    }
    let subagents = team.get("subagents").and_then(Value::as_array)?;
    if subagents.is_empty() {
        return None;
    }
    let mut instruction = String::from(
        "You are the primary agent of an ORBIT Agent Team. Delegate specialist work to the following OpenCode subagents using the task/subagent tool. Do not do all specialist work yourself.\n\n",
    );
    for subagent in subagents {
        let name = subagent
            .get("slug")
            .and_then(Value::as_str)
            .or_else(|| subagent.get("name").and_then(Value::as_str))
            .unwrap_or("subagent");
        let description = subagent
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("specialist subagent");
        instruction.push_str(&format!("- @{name}: {description}\n"));
    }
    instruction.push_str(
        "\nUse their feedback to decide the next task and repeat until the team goal is complete and verified.\n",
    );
    Some(instruction)
}

/// Returns the materialized subagent names.
pub async fn materialize(input: &Value, workspace: &Path) -> anyhow::Result<Vec<String>> {
    let Some(team) = input.get("team") else {
        return Ok(Vec::new());
    };
    if team.get("mode").and_then(Value::as_str) != Some("native") {
        return Ok(Vec::new());
    }
    let Some(subagents) = team.get("subagents").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    if subagents.is_empty() {
        return Ok(Vec::new());
    }
    let directory = workspace.join(".opencode/agents");
    tokio::fs::create_dir_all(&directory).await?;
    let mut names = Vec::new();
    for subagent in subagents {
        let name = subagent
            .get("slug")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .or_else(|| subagent.get("name").and_then(Value::as_str))
            .unwrap_or("subagent");
        let file = slug(name);
        if file.is_empty() {
            continue;
        }
        let description = subagent
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("ORBIT team subagent");
        let instructions = subagent
            .get("instructions")
            .and_then(Value::as_str)
            .unwrap_or("You are a specialist subagent.");
        let model = subagent.get("model").and_then(Value::as_str);
        let mut document = String::from("---\n");
        document.push_str(&format!(
            "description: {}\n",
            serde_json::to_string(description).unwrap_or_else(|_| "\"subagent\"".into())
        ));
        document.push_str("mode: subagent\n");
        if let Some(model) = model.filter(|value| !value.trim().is_empty()) {
            document.push_str(&format!("model: {model}\n"));
        }
        document.push_str("---\n\n");
        document.push_str(instructions);
        document.push('\n');
        tokio::fs::write(directory.join(format!("{file}.md")), document).await?;
        names.push(file);
    }
    Ok(names)
}
