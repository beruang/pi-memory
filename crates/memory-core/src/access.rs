use crate::{MemoryError, MemoryScope, MemorySensitivity};
use uuid::Uuid;

/// Represents the authenticated access context for a caller.
///
/// Enforces scope boundaries per spec Section 31:
/// - Project memory is available only within that project context.
/// - User memory is available only to the owning user.
/// - Organization memory is available only according to organization policy.
/// - Private memory requires explicit permission.
/// - Secret memory must not exist.
///
/// All MCP tool calls and API calls are validated against this context.
#[derive(Debug, Clone)]
pub struct AccessContext {
    /// Actor identifier (e.g., "agent", "cli", "api", or a user name).
    pub actor: String,
    /// If Some, restricts access to only this project's project-scope memory.
    pub project_id: Option<Uuid>,
    /// If Some, restricts access to only this user's user-scope memory.
    pub user_id: Option<Uuid>,
    /// If Some, restricts access to only this org's org-scope memory.
    pub organization_id: Option<Uuid>,
    /// Whether private-sensitivity observations are readable and writable.
    pub can_access_private: bool,
}

impl AccessContext {
    /// Default agent context: project-scoped, no user/org, no private access.
    pub fn agent(project_id: Option<Uuid>) -> Self {
        Self {
            actor: "agent".into(),
            project_id,
            user_id: None,
            organization_id: None,
            can_access_private: false,
        }
    }

    /// Full-access context for admin / CLI / debugging.
    pub fn admin() -> Self {
        Self {
            actor: "admin".into(),
            project_id: None,
            user_id: None,
            organization_id: None,
            can_access_private: true,
        }
    }

    /// User-facing API context with optional identity.
    pub fn api(actor: String, project_id: Option<Uuid>, user_id: Option<Uuid>) -> Self {
        Self {
            actor,
            project_id,
            user_id,
            organization_id: None,
            can_access_private: false,
        }
    }

    /// Validate that this context can read an observation with the given attributes.
    ///
    /// Returns `Ok(())` if access is permitted, `Err(MemoryError::AuthorizationDenied)`
    /// otherwise.
    pub fn check_read_access(
        &self,
        scope: &MemoryScope,
        obs_project_id: Option<Uuid>,
        obs_user_id: Option<Uuid>,
        obs_organization_id: Option<Uuid>,
        obs_sensitivity: &MemorySensitivity,
    ) -> Result<(), MemoryError> {
        // Secret should not exist in the system, but deny read access regardless.
        if *obs_sensitivity == MemorySensitivity::Secret {
            return Err(MemoryError::AuthorizationDenied);
        }

        // Private sensitivity requires explicit permission.
        if *obs_sensitivity == MemorySensitivity::Private && !self.can_access_private {
            return Err(MemoryError::AuthorizationDenied);
        }

        match scope {
            MemoryScope::Session => {
                // Session memory is always readable (temporary context).
                Ok(())
            }
            MemoryScope::Project => {
                // Project memory requires matching project_id.
                if let Some(ctx_pid) = self.project_id {
                    if Some(ctx_pid) != obs_project_id {
                        return Err(MemoryError::AuthorizationDenied);
                    }
                }
                Ok(())
            }
            MemoryScope::User => {
                // User memory requires matching user_id.
                if let Some(ctx_uid) = self.user_id {
                    if Some(ctx_uid) != obs_user_id {
                        return Err(MemoryError::AuthorizationDenied);
                    }
                }
                Ok(())
            }
            MemoryScope::Organization => {
                // Organization memory requires matching org_id.
                if let Some(ctx_oid) = self.organization_id {
                    if Some(ctx_oid) != obs_organization_id {
                        return Err(MemoryError::AuthorizationDenied);
                    }
                }
                Ok(())
            }
        }
    }

    /// Validate that this context can write an observation at the given scope.
    ///
    /// Returns `Ok(())` if write is permitted, `Err(MemoryError::AuthorizationDenied)`
    /// or `Err(MemoryError::SecretContentRejected)` otherwise.
    pub fn check_write_access(
        &self,
        scope: &MemoryScope,
        project_id: Option<Uuid>,
        user_id: Option<Uuid>,
        organization_id: Option<Uuid>,
        sensitivity: &MemorySensitivity,
    ) -> Result<(), MemoryError> {
        // Secret must never be written.
        if *sensitivity == MemorySensitivity::Secret {
            return Err(MemoryError::SecretContentRejected);
        }

        // Private writes require explicit permission.
        if *sensitivity == MemorySensitivity::Private && !self.can_access_private {
            return Err(MemoryError::AuthorizationDenied);
        }

        match scope {
            MemoryScope::Session => Ok(()),
            MemoryScope::Project => {
                if let Some(ctx_pid) = self.project_id {
                    if Some(ctx_pid) != project_id {
                        return Err(MemoryError::AuthorizationDenied);
                    }
                }
                Ok(())
            }
            MemoryScope::User => {
                if let Some(ctx_uid) = self.user_id {
                    if Some(ctx_uid) != user_id {
                        return Err(MemoryError::AuthorizationDenied);
                    }
                }
                Ok(())
            }
            MemoryScope::Organization => {
                if let Some(ctx_oid) = self.organization_id {
                    if Some(ctx_oid) != organization_id {
                        return Err(MemoryError::AuthorizationDenied);
                    }
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemorySensitivity;

    #[test]
    fn test_agent_can_read_project_memory() {
        let ctx = AccessContext::agent(Some(Uuid::nil()));
        assert!(ctx
            .check_read_access(
                &MemoryScope::Project,
                Some(Uuid::nil()),
                None,
                None,
                &MemorySensitivity::Internal,
            )
            .is_ok());
    }

    #[test]
    fn test_agent_cannot_read_wrong_project() {
        let ctx = AccessContext::agent(Some(Uuid::nil()));
        assert!(ctx
            .check_read_access(
                &MemoryScope::Project,
                Some(Uuid::from_u128(1)),
                None,
                None,
                &MemorySensitivity::Internal,
            )
            .is_err());
    }

    #[test]
    fn test_agent_cannot_read_private_without_permission() {
        let ctx = AccessContext::agent(None);
        assert!(ctx
            .check_read_access(
                &MemoryScope::Project,
                None,
                None,
                None,
                &MemorySensitivity::Private,
            )
            .is_err());
    }

    #[test]
    fn test_admin_can_read_private() {
        let ctx = AccessContext::admin();
        assert!(ctx
            .check_read_access(
                &MemoryScope::Project,
                None,
                None,
                None,
                &MemorySensitivity::Private,
            )
            .is_ok());
    }

    #[test]
    fn test_always_denies_secret_read() {
        let ctx = AccessContext::admin();
        assert!(ctx
            .check_read_access(
                &MemoryScope::Project,
                None,
                None,
                None,
                &MemorySensitivity::Secret,
            )
            .is_err());
    }

    #[test]
    fn test_always_denies_secret_write() {
        let ctx = AccessContext::admin();
        assert!(ctx
            .check_write_access(
                &MemoryScope::Project,
                None,
                None,
                None,
                &MemorySensitivity::Secret,
            )
            .is_err());
    }

    #[test]
    fn test_user_memory_requires_matching_user_id() {
        let ctx = AccessContext::api("user".into(), None, Some(Uuid::nil()));
        assert!(ctx
            .check_read_access(
                &MemoryScope::User,
                None,
                Some(Uuid::nil()),
                None,
                &MemorySensitivity::Internal,
            )
            .is_ok());
        assert!(ctx
            .check_read_access(
                &MemoryScope::User,
                None,
                Some(Uuid::from_u128(1)),
                None,
                &MemorySensitivity::Internal,
            )
            .is_err());
    }

    #[test]
    fn test_session_memory_always_accessible() {
        let ctx = AccessContext::agent(None);
        assert!(ctx
            .check_read_access(
                &MemoryScope::Session,
                None,
                None,
                None,
                &MemorySensitivity::Internal,
            )
            .is_ok());
    }

    #[test]
    fn test_write_private_requires_permission() {
        let ctx = AccessContext::agent(None);
        assert!(ctx
            .check_write_access(
                &MemoryScope::Project,
                None,
                None,
                None,
                &MemorySensitivity::Private,
            )
            .is_err());

        let admin = AccessContext::admin();
        assert!(admin
            .check_write_access(
                &MemoryScope::Project,
                None,
                None,
                None,
                &MemorySensitivity::Private,
            )
            .is_ok());
    }
}
