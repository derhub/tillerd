use crate::context::Ctx;
use crate::entities::session::SessionId;
use crate::infra::session::SessionRepo;
use crate::shared::cqs::Query;
use crate::shared::errors::{Error, Result};

/// Return the session's launch spec (the recipe + placements).
pub struct GetLaunchSpec {
    pub id: SessionId,
}

impl Query<Ctx> for GetLaunchSpec {
    type Out = Option<crate::entities::launch_spec::LaunchSpec>;
    async fn handle(&self, cx: &Ctx) -> Result<Self::Out> {
        let s = SessionRepo::get(cx.db(), &self.id)
            .await?
            .ok_or_else(|| Error::SessionNotFound(self.id.as_str().to_owned()))?;

        match (s.spec_version, s.spec_json) {
            (Some(v), Some(j)) => {
                let spec = crate::entities::launch_spec::migrate(&j, v)
                    .map_err(|e| Error::Validation {
                        field: "spec",
                        reason: e.to_string(),
                    })?
                    .0;
                Ok(Some(spec))
            }
            _ => Ok(None),
        }
    }
}
