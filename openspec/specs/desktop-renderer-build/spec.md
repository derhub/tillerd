# desktop-renderer-build

## Purpose

The shared renderer as a static single-page application serving both the web deployment and the
desktop web view, with the desktop package carrying only the native shell and build wiring (no
scaffold renderer), and client-side routing resolving under the web view's asset origin.

## Requirements

### Requirement: Renderer builds as a static single-page application

The renderer SHALL build into static client assets that load without a server-side rendering
runtime, so the native web view can serve them directly from the bundle. The same static build
SHALL serve both the desktop application and the web deployment; there is no renderer fork.

#### Scenario: Producing the static renderer build

- **WHEN** the renderer build runs
- **THEN** it emits static client assets (markup, scripts, styles) with no server-render entry
- **AND** the assets load and run without a Node server-render process

#### Scenario: Single renderer for both hosts

- **WHEN** either the desktop application or the web deployment loads the renderer
- **THEN** both load the same static client build, with no behavioral fork in the user-facing
  interface

### Requirement: Desktop application loads the renderer build, not a scaffold template

The desktop application SHALL load the shared renderer build as its frontend, and SHALL NOT
contain a rival renderer, entry document, or component tree of its own. The default
project-scaffold renderer (template entry document, sample component, sample assets) SHALL be
removed so the desktop package carries only the native shell and the build wiring.

#### Scenario: Desktop frontend resolves to the shared renderer

- **WHEN** the desktop application's production frontend is resolved at build time
- **THEN** it resolves to the shared renderer's static client build output
- **AND** no scaffold-template renderer is present in the desktop package

#### Scenario: No duplicate renderer entry

- **WHEN** the desktop package is inspected
- **THEN** it contains no second renderer entry document, root component, or sample assets
  distinct from the shared renderer

### Requirement: Development run serves the shared renderer with live reload

In development, the desktop application SHALL load the shared renderer from its development
server with live reload, rather than a scaffold-local development server.

#### Scenario: Launching the desktop application in development

- **WHEN** a developer starts the desktop application in development mode
- **THEN** the native window loads the shared renderer from its development server
- **AND** edits to the renderer are reflected via live reload without restarting the native shell

### Requirement: Production build orchestrates the renderer build before bundling

The desktop production build SHALL build the shared renderer to its static client output before
producing the native bundle, so the bundle embeds the current renderer build.

#### Scenario: Producing a desktop bundle

- **WHEN** the desktop production build runs
- **THEN** the shared renderer is built to its static client output first
- **AND** the native bundle embeds that output as its frontend

### Requirement: Client-side routing resolves under the web view

The renderer's client-side routes SHALL resolve when the application is loaded over the native
web view's asset origin, including deep links and reloads, without a server to rewrite routes.

#### Scenario: Navigating a client route in the desktop application

- **WHEN** the user navigates to or reloads a client-side route in the desktop application
- **THEN** the route resolves and renders within the web view
- **AND** no server route rewrite is required
