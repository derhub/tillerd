use serde::Deserialize;

use crate::app::session::LaunchSpecView;
use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::errors::{Error, Result};
use crate::shared::message::Query;

/// Return the session's launch spec (the recipe + placements).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLaunchSpec {
    pub id: String,
}

impl Query<Ctx> for GetLaunchSpec {
    type Out = Option<LaunchSpecView>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let id = SessionId::from_string(&self.id);
        let s = SessionRepo::get(cx.db(), &id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.clone()))?;

        match (s.spec_version, s.spec_json) {
            (Some(v), Some(j)) => {
                let spec = crate::entities::launch_spec::migrate(&j, v)
                    .map_err(|e| Error::Validation {
                        field: "spec",
                        reason: e.to_string(),
                    })?
                    .0;
                Ok(Some(LaunchSpecView(spec)))
            }
            _ => Ok(None),
        }
    }
}
