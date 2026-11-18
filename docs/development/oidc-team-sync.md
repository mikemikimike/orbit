# OIDC group → access-team sync

In addition to org roles, OIDC groups can add users to access teams (enterprise `teams`
feature).

```yaml
auth:
  oidc:
    roleClaim: groups
    teamGroups:
      platform: platform:member
      platform-leads: platform:lead
```

Environment form:

```bash
ORBIT_OIDC_TEAM_GROUPS='platform=platform:member,platform-leads=platform:lead'
# or JSON
ORBIT_OIDC_TEAM_GROUPS='{"platform":"platform:member"}'
```

- Values are `access-team-slug:role`, role `lead|member|viewer`.
- Sync runs on authenticated OIDC requests and is additive; it never removes a membership.
- Missing groups claim does nothing.
- Missing team slug is logged and skipped.
- Requires the `teams` license feature.
- Org role mapping (`groupRoles`) is independent and continues to work.
