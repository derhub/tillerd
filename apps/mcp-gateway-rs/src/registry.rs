//! Registry: in-memory, generation counter, no persistence.

use rmcp::model::{Prompt, Resource, Tool};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::router;

#[derive(Default)]
struct Inner {
    tools_by_backend: HashMap<String, Vec<Tool>>,
    tool_owner: HashMap<String, String>,
    prompts_by_backend: HashMap<String, Vec<Prompt>>,
    resources_by_backend: HashMap<String, Vec<Resource>>,
    resource_owner: HashMap<String, String>,
}

#[derive(Clone, Default)]
pub struct Registry {
    inner: Arc<RwLock<Inner>>,
    generation: Arc<AtomicU64>,
}

impl Registry {
    pub fn set_backend(
        &self,
        backend: &str,
        tools: Vec<Tool>,
        prompts: Vec<Prompt>,
        resources: Vec<Resource>,
        allowed: Option<&[String]>,
    ) {
        let mut w = self.inner.write().unwrap();
        w.tool_owner.retain(|_, b| b != backend);
        w.resource_owner.retain(|_, b| b != backend);

        let mut ns_tools = Vec::new();
        for mut t in tools {
            let admit =
                allowed.is_none_or(|list| list.iter().any(|a| a.as_str() == t.name.as_ref()));
            if !admit {
                continue;
            }
            let public = router::namespaced(backend, &t.name);
            w.tool_owner.insert(public.clone(), backend.to_string());
            t.name = public.into();
            ns_tools.push(t);
        }
        w.tools_by_backend.insert(backend.to_string(), ns_tools);

        let ns_prompts = prompts
            .into_iter()
            .map(|mut p| {
                p.name = router::namespaced(backend, &p.name);
                p
            })
            .collect();
        w.prompts_by_backend.insert(backend.to_string(), ns_prompts);

        for r in &resources {
            w.resource_owner.insert(r.uri.clone(), backend.to_string());
        }
        w.resources_by_backend
            .insert(backend.to_string(), resources);

        drop(w);
        self.bump();
    }

    pub fn drop_backend(&self, backend: &str) {
        let mut w = self.inner.write().unwrap();
        let had = w.tools_by_backend.remove(backend).is_some()
            | w.prompts_by_backend.remove(backend).is_some()
            | w.resources_by_backend.remove(backend).is_some();
        w.tool_owner.retain(|_, b| b != backend);
        w.resource_owner.retain(|_, b| b != backend);
        drop(w);
        if had {
            self.bump();
        }
    }

    pub fn all_tools(&self) -> Vec<Tool> {
        self.inner
            .read()
            .unwrap()
            .tools_by_backend
            .values()
            .flatten()
            .cloned()
            .collect()
    }

    pub fn all_prompts(&self) -> Vec<Prompt> {
        self.inner
            .read()
            .unwrap()
            .prompts_by_backend
            .values()
            .flatten()
            .cloned()
            .collect()
    }

    pub fn all_resources(&self) -> Vec<Resource> {
        self.inner
            .read()
            .unwrap()
            .resources_by_backend
            .values()
            .flatten()
            .cloned()
            .collect()
    }

    pub fn owner_of(&self, public: &str) -> Option<String> {
        self.inner.read().unwrap().tool_owner.get(public).cloned()
    }

    pub fn resource_owner_of(&self, uri: &str) -> Option<String> {
        self.inner.read().unwrap().resource_owner.get(uri).cloned()
    }

    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    fn bump(&self) {
        self.generation.fetch_add(1, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn tool(name: &str) -> Tool {
        Tool::new(name.to_string(), "t", Arc::new(serde_json::Map::new()))
    }
    fn prompt(name: &str) -> Prompt {
        serde_json::from_value(serde_json::json!({ "name": name })).unwrap()
    }
    fn resource(uri: &str) -> Resource {
        serde_json::from_value(serde_json::json!({ "uri": uri, "name": uri })).unwrap()
    }

    #[test]
    fn namespaces_tools_and_prompts_but_keeps_resource_uris() {
        let r = Registry::default();
        r.set_backend(
            "gh",
            vec![tool("create_issue")],
            vec![prompt("review")],
            vec![resource("file:///x")],
            None,
        );
        assert_eq!(r.owner_of("gh__create_issue").as_deref(), Some("gh"));
        assert_eq!(r.all_prompts()[0].name, "gh__review");
        assert_eq!(r.all_resources()[0].uri, "file:///x");
        assert_eq!(r.resource_owner_of("file:///x").as_deref(), Some("gh"));
    }

    #[test]
    fn generation_advances_on_change() {
        let r = Registry::default();
        let g0 = r.generation();
        r.set_backend("gh", vec![tool("a")], vec![], vec![], None);
        assert!(r.generation() > g0);
    }

    #[test]
    fn allowlist_filters_tools_only() {
        let r = Registry::default();
        let allowed = vec!["a".to_string()];
        r.set_backend(
            "gh",
            vec![tool("a"), tool("b")],
            vec![prompt("p")],
            vec![],
            Some(&allowed),
        );
        assert!(r.owner_of("gh__a").is_some());
        assert!(r.owner_of("gh__b").is_none());
        assert_eq!(r.all_prompts().len(), 1);
    }

    #[test]
    fn dropped_backend_disappears_everywhere() {
        let r = Registry::default();
        r.set_backend(
            "gh",
            vec![tool("a")],
            vec![prompt("p")],
            vec![resource("u")],
            None,
        );
        r.drop_backend("gh");
        assert!(r.all_tools().is_empty());
        assert!(r.all_prompts().is_empty());
        assert!(r.all_resources().is_empty());
        assert!(r.resource_owner_of("u").is_none());
    }
}
