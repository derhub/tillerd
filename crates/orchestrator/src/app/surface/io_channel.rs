// Re-export off-bus fns from common for callers that import via this module.
pub use super::common::{attach_surface, resize_surface, send_surface_input};

#[cfg(test)]
mod tests {
    use crate::app::surface::common::{attach_surface, resize_surface, send_surface_input};
    use crate::app::surface::test_util::harness;
    use crate::entities::SurfaceId;
    use crate::infra::runtime::RuntimeCall;

    // Scenario: Surface input is an I/O channel -- off the bus, no command object
    #[tokio::test]
    async fn input_forwards_to_the_runtime_off_bus() {
        let h = harness().await;
        let id = SurfaceId::from_string("io");

        send_surface_input(h.bus.cx(), id.as_str(), b"ls\n")
            .await
            .unwrap();

        assert_eq!(
            h.runtime.calls(),
            vec![RuntimeCall::Input {
                surface: id,
                bytes: b"ls\n".to_vec(),
            }]
        );
    }

    #[tokio::test]
    async fn resize_forwards_to_the_runtime_off_bus() {
        let h = harness().await;
        let id = SurfaceId::from_string("io");

        resize_surface(h.bus.cx(), id.as_str(), 120, 40)
            .await
            .unwrap();

        assert_eq!(
            h.runtime.calls(),
            vec![RuntimeCall::Resize {
                surface: id,
                cols: 120,
                rows: 40,
            }]
        );
    }

    // Scenario: attach is lazy/per-surface, off the bus
    #[tokio::test]
    async fn attach_forwards_to_the_runtime_off_bus() {
        let h = harness().await;
        let id = SurfaceId::from_string("io");

        attach_surface(h.bus.cx(), id.as_str()).await.unwrap();

        assert_eq!(h.runtime.calls(), vec![RuntimeCall::Attach(id)]);
    }
}
