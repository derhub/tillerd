use serde::Deserialize;

use crate::app::settings::ProfileView;
use crate::context::Ctx;
use crate::infra::config::ProfileStore;
use crate::shared::message::Query;
use crate::shared::Result;

/// One profile by id; `None` when it does not exist.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetProfile {
    pub id: String,
}

impl Query<Ctx> for GetProfile {
    type Out = Option<ProfileView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        Ok(ProfileStore::new(cx.fs_root())
            .get(&self.id)
            .await?
            .map(ProfileView))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boot::test_ctx;
    use crate::shared::message::Command;

    #[tokio::test]
    async fn returns_the_profile_after_create_and_none_for_a_missing_id() {
        let cx = test_ctx().await.unwrap();
        crate::app::settings::NewProfile {
            id: "pf_1".to_owned(),
            name: "Work".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();

        let found = GetProfile {
            id: "pf_1".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();
        assert_eq!(found.map(|p| p.0.name), Some("Work".to_owned()));

        let missing = GetProfile {
            id: "pf_none".to_owned(),
        }
        .handle(&cx)
        .await
        .unwrap();
        assert!(missing.is_none());
    }
}
