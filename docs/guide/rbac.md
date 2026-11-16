# RBAC

## Built-in roles

| Role | Can do |
|---|---|
| `owner` | everything, including `settings:write` |
| `admin` | everything except platform settings |
| `operator` | create/cancel executions, write agents/connections/triggers |
| `viewer` | read-only |

Permissions are checked in handlers with `auth.require(Permission::…)`. The platform
evaluates them from the stored role; custom roles carry an explicit permission array.

## Custom roles (enterprise)

`GET/POST /api/v1/roles`, `PATCH/DELETE /api/v1/roles/{id}`; mutations require
`settings:write` and the `custom_roles` license feature. Built-in role names are reserved;
permissions are validated against the catalog and audited. Assign with
`PATCH /api/v1/users/{id} {"role":"Release manager"}`.

The role name is stored on the user; at authentication time ORBIT resolves built-in
permissions or the custom permission set. Deleting a role locks assigned users out until
they are reassigned (403), rather than silently downgrading them.

## OIDC group mapping

`roleClaim` + `groupRoles` map IdP groups to built-in roles; `syncRoles` re-evaluates on
login. See [docs/development/oidc.md](../development/oidc.md).

## Not implemented yet

Resource-level permissions (per agent/connection/team), shared connections with scoped
access, and marketplace entitlements. See #15.
