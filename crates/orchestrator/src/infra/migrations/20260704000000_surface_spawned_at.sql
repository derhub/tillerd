-- Additive under the 0.0.6 data-model freeze (ADR-0032): a nullable spawn
-- timestamp for the panel title's elapsed-since-spawn display
-- (ui-panel-compound spec). NULL until the PTY behind the row is first
-- confirmed running; a surface that never spawned (or was created but not
-- yet confirmed) has nothing to show.
ALTER TABLE surface ADD COLUMN spawned_at INTEGER;
