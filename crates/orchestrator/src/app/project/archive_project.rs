use crate::context::Ctx;
use crate::entities::project::{ProjectId, ProjectStatus};
use crate::infra::project::ProjectRepo;
use crate::infra::session::SessionRepo;
use crate::shared::pagination::Page;
use crate::shared::{Command, Error, Result};

/// Archive a project (cascades to its sessions; rejected unless every session is idle).
pub struct ArchiveProject {
    pub id: ProjectId,
}

impl Command<Ctx> for ArchiveProject {
    async fn handle(&self, cx: &Ctx) -> Result<()> {
        let project = ProjectRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::ProjectNotFound(self.id.as_str().to_owned()))?;
        project.guard_not_unfiled()?;
        project.guard_active()?;

        // Archive-requires-idle: count live surfaces across all sessions in this project.
        let live_count = ProjectRepo::count_live_surfaces(cx.db(), &self.id).await?;
        if live_count > 0 {
            return Err(Error::SessionNotIdle(self.id.as_str().to_owned()));
        }

        cx.transaction(async |tx| {
            // Cascade: archive all active sessions.
            let sessions = SessionRepo::list(&mut **tx, &self.id, Page::All).await?;
            let now = crate::shared::datetime::now_iso8601();
            for session in sessions.items {
                if session.status == crate::entities::session::SessionStatus::Active {
                    SessionRepo::set_archived(&mut **tx, &session.id, &now).await?;
                }
            }
            // Archive the project itself.
            let mut p = project.clone();
            p.status = ProjectStatus::Archived;
            ProjectRepo::update(&mut **tx, &p).await
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::project::test_util::*;

    use super::super::get_project_by_id::GetProjectById;

    #[tokio::test]
    async fn archive_project_is_rejected_when_sessions_have_live_surfaces() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-arch-live", "LiveProject", &default_ws()).await;

        // Seed a session and a live surface.
        sqlx::query("INSERT INTO session (id, project_id, title) VALUES (?, ?, ?)")
            .bind("s-live-1")
            .bind("p-arch-live")
            .bind("test")
            .execute(ctx.db())
            .await
            .unwrap();
        sqlx::query("INSERT INTO surface (id, session_id, kind, status) VALUES (?, ?, ?, ?)")
            .bind("surf-live-1")
            .bind("s-live-1")
            .bind("terminal")
            .bind("live")
            .execute(ctx.db())
            .await
            .unwrap();

        let result = bus
            .execute(ArchiveProject {
                id: ProjectId::new("p-arch-live"),
            })
            .await;
        assert!(
            matches!(result, Err(Error::SessionNotIdle(_))),
            "archive must be rejected when live surfaces exist: {result:?}"
        );
    }

    #[tokio::test]
    async fn archive_project_cascades_to_sessions_when_idle() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-arch-idle", "IdleProject", &default_ws()).await;

        // Seed a session with no live surfaces.
        sqlx::query("INSERT INTO session (id, project_id, title) VALUES (?, ?, ?)")
            .bind("s-idle-1")
            .bind("p-arch-idle")
            .bind("test")
            .execute(ctx.db())
            .await
            .unwrap();

        bus.execute(ArchiveProject {
            id: ProjectId::new("p-arch-idle"),
        })
        .await
        .unwrap();

        let project = bus
            .query(GetProjectById {
                id: ProjectId::new("p-arch-idle"),
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(project.status, ProjectStatus::Archived);
    }

    #[tokio::test]
    async fn archive_unfiled_project_is_rejected() {
        let (_ctx, bus) = ctx().await;
        let result = bus
            .execute(ArchiveProject {
                id: unfiled_project_id(),
            })
            .await;
        assert!(
            matches!(result, Err(Error::ProjectIsUnfiled)),
            "unfiled must not be archived: {result:?}"
        );
    }

    #[tokio::test]
    async fn archive_already_archived_project_is_rejected() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-arch-2", "AlreadyArchived", &default_ws()).await;

        bus.execute(ArchiveProject {
            id: ProjectId::new("p-arch-2"),
        })
        .await
        .unwrap();

        let result = bus
            .execute(ArchiveProject {
                id: ProjectId::new("p-arch-2"),
            })
            .await;
        assert!(
            matches!(result, Err(Error::ProjectAlreadyArchived)),
            "re-archiving must be rejected: {result:?}"
        );
    }

    #[tokio::test]
    async fn archive_project_cascade_archives_sessions_atomically() {
        let (ctx, bus) = ctx().await;
        seed_project(ctx.db(), "p-cas", "Cascade", &default_ws()).await;

        // Seed two sessions with no live surfaces.
        for i in 0..2u32 {
            sqlx::query("INSERT INTO session (id, project_id, title) VALUES (?, ?, ?)")
                .bind(format!("s-cas-{i}"))
                .bind("p-cas")
                .bind(format!("Session {i}"))
                .execute(ctx.db())
                .await
                .unwrap();
        }

        bus.execute(ArchiveProject {
            id: ProjectId::new("p-cas"),
        })
        .await
        .unwrap();

        // Both sessions must be archived.
        use crate::entities::session::SessionStatus;
        for i in 0..2u32 {
            let session = SessionRepo::get(
                ctx.db(),
                &crate::entities::session::SessionId::from_string(format!("s-cas-{i}")),
            )
            .await
            .unwrap()
            .unwrap();
            assert_eq!(
                session.status,
                SessionStatus::Archived,
                "session {i} must be archived"
            );
        }
    }
}
