use serde::Deserialize;

use crate::app::template::LaunchTemplateView;
use crate::context::Ctx;
use crate::shared::message::Query;
use crate::shared::Result;

/// Fetch one launch template by id.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLaunchTemplateById {
    pub id: String,
}

impl Query<Ctx> for GetLaunchTemplateById {
    type Out = Option<LaunchTemplateView>;

    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        Ok(sqlx::query_as::<_, LaunchTemplateView>(
            "SELECT id, project_id, spec_version, spec_json
             FROM launch_template
             WHERE id = ?",
        )
        .bind(&self.id)
        .fetch_optional(cx.db())
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::template::test_util::*;
    use crate::entities::ProjectId;

    use super::super::list_launch_templates_by_project::ListLaunchTemplatesByProject;
    use super::super::new_launch_template_cmd::NewLaunchTemplateCmd;

    #[tokio::test]
    async fn get_launch_template_by_id_returns_created_template() {
        let dir = tempfile::TempDir::new().unwrap();
        let (cx, bus) = ctx(&dir).await;

        bus.execute(NewLaunchTemplateCmd {
            id: uuid::Uuid::new_v4().to_string(),
            project_id: ProjectId::new(UNFILED).as_str().to_owned(),
            spec_version: 1,
            spec_json: r#"{"items":[]}"#.to_owned(),
        })
        .await
        .unwrap();

        let listing = bus
            .query(ListLaunchTemplatesByProject {
                project_id: UNFILED.to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(listing.items.len(), 1);
        let id = listing.items[0].id.clone();

        let got = bus
            .query(GetLaunchTemplateById { id: id.clone() })
            .await
            .unwrap();

        assert!(got.is_some());
        let tmpl = got.unwrap();
        assert_eq!(tmpl.id, id);
        assert_eq!(tmpl.spec_version, 1);
        assert_eq!(tmpl.spec_json, r#"{"items":[]}"#);

        // query wrote nothing -- count still 1
        let listing2 = bus
            .query(ListLaunchTemplatesByProject {
                project_id: UNFILED.to_owned(),
                limit: None,
                offset: None,
                after: None,
            })
            .await
            .unwrap();
        assert_eq!(listing2.items.len(), 1);
        let _ = cx;
    }
}
