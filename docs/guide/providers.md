# Providers and models

ORBIT supports two provider kinds:

| Kind | Auth | Use |
|---|---|---|
| `openai_compatible` | ORBIT stores an encrypted credential and injects it into the harness | vLLM, LiteLLM, gateways, OpenRouter-style endpoints |
| `harness_managed` | The harness owns credentials (e.g. `opencode auth export`) | Provider inventories managed by OpenCode/Pi |

## Models

`settings.models` is the model catalog used by agent and worker selectors. The provider
**Test connection** button probes `/models` (when `compat.modelsEndpoint` is enabled),
fills the catalog and default model, and saved providers persist the discovered list.
Endpoints without `/models` can use a free-text model.

## Using OpenCode as a platform provider

Create an `openai_compatible` provider with the OpenCode gateway base URL and API key if
you want ORBIT-managed credentials instead of a harness auth Secret.

## Decoupling from the agent

The agent stores a default harness/model. Work and team runs can override the harness for
that run; model selection can be per agent, per team step, or worker-profile-level. There
is no hardcoded provider or model inventory in the control plane.
