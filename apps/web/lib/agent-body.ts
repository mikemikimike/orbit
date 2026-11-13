import type { components } from "@orbit/api-client";
import type { AgentDefinition } from "@orbit/contracts";

/** Generated OpenAPI type for the agent definition body. */
export type AgentDefinitionBody = components["schemas"]["AgentDefinition"];

/**
 * The Zod contract in `@orbit/contracts` and the generated OpenAPI schema describe the
 * same `orbit.io/v1` document, but their optional/required fields differ. Convert once
 * here instead of scattering `as never` casts through pages.
 */
export function toAgentDefinitionBody(definition: AgentDefinition): AgentDefinitionBody {
  return definition as unknown as AgentDefinitionBody;
}
