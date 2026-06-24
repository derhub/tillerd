import { GlobalRegistrator } from "@happy-dom/global-registrator";

GlobalRegistrator.register();

// happy-dom lacks Web Animations API. base-ui ScrollArea calls getAnimations() in a ResizeObserver callback; the missing method throws on every observe and base-ui retries, triggering React "Maximum update depth exceeded". Stub returns empty list so thumb-geometry completes.
if (typeof Element !== "undefined" && !Element.prototype.getAnimations) {
  Element.prototype.getAnimations = () => [];
}
if (typeof Document !== "undefined" && !Document.prototype.getAnimations) {
  Document.prototype.getAnimations = () => [];
}
