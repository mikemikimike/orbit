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
- Missing team slug is logged and skipped by default.
- With `autoCreateTeams: true` (or `ORBIT_OIDC_AUTO_CREATE_TEAMS=true`), a missing
  team slug is created automatically using the IdP group name and mapped role.
- Requires the `teams` license feature.
- Org role mapping (`groupRoles`) is independent and continues to work.

Example for a `Devs` IdP group:

```yaml
auth:
  oidc:
    teamGroups:
      Devs: devs:member
    autoCreateTeams: true
```

The first login creates access team `devs` and adds the user as `member`.

## Deny unmapped users

Set `requireMappedGroup: true` (or `ORBIT_OIDC_REQUIRE_MAPPED_GROUP=true`) to deny users
whose groups match neither `groupRoles` nor `teamGroups`. They receive a clear
"No access to ORBIT" page instead of being auto-provisioned as viewers. Existing users
with no mapped group are denied on their next request while this is enabled.
