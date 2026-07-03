use orchestrator::app::template::{
    ApplyTemplateSpec, DiscardLaunchTemplate, DiscardTemplate, ExportTemplate,
    GetLaunchTemplateById, GetTemplateById, ImportTemplate, LaunchTemplateView,
    ListLaunchTemplatesByProject, ListTemplates, NewLaunchTemplateCmd, PinTemplate, TemplateView,
    UnpinTemplate,
};

use uuid::Uuid;

use crate::transport::macros::{transport_command, transport_create, transport_query};

transport_create!(
    launch_template_create(
        project_id: String,
        spec_version: u32,
        spec_json: String,
    ) -> LaunchTemplateView {
        let id = Uuid::new_v4().to_string();
        execute: NewLaunchTemplateCmd {
            id: id.clone(),
            project_id,
            spec_version,
            spec_json,
        },
        read_back: GetLaunchTemplateById { id: id.clone() },
        map: |t| t,
        missing: "launch template vanished after create",
    }
);

transport_query!(
    launch_template_list(
        project_id: String,
        limit: Option<u32>,
        offset: Option<u32>,
        after: Option<String>,
    ) -> Vec<LaunchTemplateView>
        => ListLaunchTemplatesByProject { project_id, limit, offset, after },
        |listing| listing.items
);

transport_query!(
    launch_template_get(id: String) -> Option<LaunchTemplateView>
        => GetLaunchTemplateById { id },
        |t| t
);

transport_command!(launch_template_discard(id: String) => DiscardLaunchTemplate { id });

transport_command!(
    launch_template_apply_spec(id: String, spec_version: u32, spec_json: String)
        => ApplyTemplateSpec { id, spec_version, spec_json }
);

transport_query!(
    template_list() -> Vec<TemplateView>
        => ListTemplates,
        |templates| templates
);

transport_query!(
    template_get(id: String) -> Option<TemplateView>
        => GetTemplateById { id },
        |t| t
);

transport_command!(
    template_import(name: String, spec_version: u32, spec_json: String)
        => ImportTemplate { name, spec_version, spec_json }
);

transport_command!(
    template_export(id: String, dest_path: String)
        => ExportTemplate { id, dest_path }
);

transport_command!(template_discard(id: String) => DiscardTemplate { id });

transport_command!(template_pin(id: String) => PinTemplate { id });

transport_command!(template_unpin(id: String) => UnpinTemplate { id });

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_keys(value: &serde_json::Value, expected: &[&str]) {
        let obj = value.as_object().expect("response serializes to an object");
        let mut got: Vec<&str> = obj.keys().map(String::as_str).collect();
        got.sort_unstable();
        let mut want = expected.to_vec();
        want.sort_unstable();
        assert_eq!(got, want, "response keys drifted from the SDK contract");
    }

    #[test]
    fn launch_template_response_matches_sdk_launch_template_shape() {
        let t = LaunchTemplateView {
            id: "t".into(),
            project_id: "p".into(),
            spec_version: 1,
            spec_json: "{}".into(),
        };
        assert_keys(
            &serde_json::to_value(t).unwrap(),
            &["id", "projectId", "specVersion", "specJson"],
        );
    }

    #[test]
    fn template_response_matches_sdk_template_shape() {
        let t = TemplateView {
            id: "t".into(),
            name: "T".into(),
            origin: "custom".into(),
            pinned: false,
            spec_version: 1,
            spec_json: "{}".into(),
        };
        assert_keys(
            &serde_json::to_value(t).unwrap(),
            &["id", "name", "origin", "pinned", "specVersion", "specJson"],
        );
    }
}
