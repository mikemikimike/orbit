use orbit_domain::UserRole;
use serde::{Deserialize, Serialize};

/// Coarse-grained platform permissions checked in handlers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    AgentsRead,
    AgentsWrite,
    ExecutionsRead,
    ExecutionsCreate,
    ExecutionsCancel,
    ApprovalsDecide,
    RuntimesRead,
    ProvidersRead,
    ProvidersWrite,
    ConnectionsRead,
    ConnectionsWrite,
    TriggersWrite,
    SettingsRead,
    SettingsWrite,
    MembersRead,
    AuditRead,
}

impl Permission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AgentsRead => "agents:read",
            Self::AgentsWrite => "agents:write",
            Self::ExecutionsRead => "executions:read",
            Self::ExecutionsCreate => "executions:create",
            Self::ExecutionsCancel => "executions:cancel",
            Self::ApprovalsDecide => "approvals:decide",
            Self::RuntimesRead => "runtimes:read",
            Self::ProvidersRead => "providers:read",
            Self::ProvidersWrite => "providers:write",
            Self::ConnectionsRead => "connections:read",
            Self::ConnectionsWrite => "connections:write",
            Self::TriggersWrite => "triggers:write",
            Self::SettingsRead => "settings:read",
            Self::SettingsWrite => "settings:write",
            Self::MembersRead => "members:read",
            Self::AuditRead => "audit:read",
        }
    }

    pub fn allows(role: UserRole, permission: Permission) -> bool {
        use Permission::*;
        match role {
            UserRole::Viewer => matches!(
                permission,
                AgentsRead
                    | ExecutionsRead
                    | RuntimesRead
                    | ProvidersRead
                    | ConnectionsRead
                    | SettingsRead
                    | MembersRead
            ),
            UserRole::Operator => matches!(
                permission,
                AgentsRead
                    | AgentsWrite
                    | ExecutionsRead
                    | ExecutionsCreate
                    | ExecutionsCancel
                    | RuntimesRead
                    | ProvidersRead
                    | ConnectionsRead
                    | ConnectionsWrite
                    | TriggersWrite
                    | SettingsRead
                    | MembersRead
            ),
            UserRole::Admin => !matches!(permission, SettingsWrite),
            UserRole::Owner => true,
        }
    }

    pub fn all_for(role: UserRole) -> Vec<&'static str> {
        const ALL: [Permission; 16] = [
            Permission::AgentsRead,
            Permission::AgentsWrite,
            Permission::ExecutionsRead,
            Permission::ExecutionsCreate,
            Permission::ExecutionsCancel,
            Permission::ApprovalsDecide,
            Permission::RuntimesRead,
            Permission::ProvidersRead,
            Permission::ProvidersWrite,
            Permission::ConnectionsRead,
            Permission::ConnectionsWrite,
            Permission::TriggersWrite,
            Permission::SettingsRead,
            Permission::SettingsWrite,
            Permission::MembersRead,
            Permission::AuditRead,
        ];
        ALL.into_iter()
            .filter(|p| Permission::allows(role, *p))
            .map(|p| p.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewers_cannot_write() {
        assert!(Permission::allows(UserRole::Viewer, Permission::AgentsRead));
        assert!(!Permission::allows(
            UserRole::Viewer,
            Permission::AgentsWrite
        ));
        assert!(Permission::allows(
            UserRole::Operator,
            Permission::ExecutionsCreate
        ));
        assert!(!Permission::allows(
            UserRole::Operator,
            Permission::ProvidersWrite
        ));
        assert!(Permission::allows(
            UserRole::Admin,
            Permission::ProvidersWrite
        ));
        assert!(Permission::allows(
            UserRole::Owner,
            Permission::SettingsWrite
        ));
    }
}
