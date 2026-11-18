# Access teams and sharing

Access teams are the multi-team ownership model for resources. They are **not** Agent
Teams (workflows); see [agent-teams.md](agent-teams.md).

## Teams

- **Teams** in the sidebar lists access teams; create one and add members as `lead`,
  `member` or `viewer`. The creator becomes lead.
- The **Default** team is created with the organization and cannot be deleted. Existing
  resources belong to it.
- Org owners/admins govern all teams. Team leads manage their team's members and
  resources.
- Team management is an enterprise feature (`teams`). Community installs keep the single
  Default team and org-wide visibility.

## Visibility

Every shareable resource (agent, skill, connection, provider, worker profile, trigger,
tool policy, Agent Team) has `ownerUserId`, `ownerTeamId` and `visibility`:

| Visibility | Who can see it |
|---|---|
| `private` | owner and org owner/admin |
| `team` | members of the owning team, plus org owner/admin |
| `org` | all organization members |

A team worker only claims executions whose agent is `org`-visible or owned by its team.
Execution context, events, cancellation and approvals use the same rule.

## Sharing

Open a resource and choose **Share** (or call
`GET/PATCH /api/v1/sharing/{resource}/{id}`). Changing visibility requires manage access
and the `resource_permissions` enterprise feature; changes are audited. Existing data
stays readable if a license expires — only new team/share writes are blocked.

## Strict team isolation

Enabled by default (`Settings → Access policy`, API `GET/PUT /api/v1/settings/access`).
When enabled, non-admin users only see resources owned by their access teams (or their own
private resources). Organization visibility no longer exposes a team-owned resource to
other teams. Org owners/admins still see everything.

Existing users are migrated into the **Default** team, so legacy Default-owned resources
remain visible while non-default teams stay isolated.
