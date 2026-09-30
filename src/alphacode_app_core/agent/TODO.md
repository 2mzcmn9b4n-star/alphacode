# Alphacode Improvement Roadmap

## UI Components
- [x] Theme system (25+ curated presets with smooth transitions)
- [x] Prompt entry animation (Gaussian bell-curve easing, shimmer effects)
- [x] Rainbow prompt colors (smooth HSL-based hue cycling)
- [x] Animated tool colors (continuous hue rotation)
- [x] Dialog system (modern, accessible)
- [x] Chat interface (streaming, state)
- [x] Settings panel (categorized, persistent)
- [x] About screen
- [x] Context menu (right-click actions)

## Core
- [x] Client API layer
- [x] State management
- [x] Event system
- [x] Session handling

## Styling
- [x] Theme system (dark/light with 30+ presets)
- [x] Smooth theme transitions (perceptual color blending)
- [x] Color utilities (brighten, dim, with_alpha, smooth transitions)
- [x] CSS/Stylesheets
- [x] Responsive layout
- [x] Accessibility (ARIA, keyboard)

## New Premium Themes Added
- [x] Tokyo Night Storm
- [x] Monokai Pro Spectrum
- [x] Oxocarbon Dark
- [x] Modus Vivendi
- [x] Everblush
- [x] Penumbra Dark
- [x] Nova Dark
- [x] Quiet Light

## Performance Improvements
- [x] HSL-based color calculations for smooth animations
- [x] Gaussian easing for prompt entry (smoother than linear)
- [x] Perceptual color blending (smoothstep)
- [x] Component tests
- [x] Integration tests
- [x] Accessibility tests

## Todo System
- [x] Core todo tool with CRUD operations
- [x] Todo types (TodoItem, TodoGoal, TodoPlan)
- [x] Confidence tracking (planning + completion)
- [x] Confidence history (tool-maintained trail)
- [x] Goal-level assessments (feedback loop, ownership)
- [x] Plan-level intent assessment
- [x] Quality gates (intent, feedback loop, ownership, completion)
- [x] Gate observations (deferred to turn-end digest)
- [x] Auto-poke for incomplete todos
- [x] Spike detection (confidence jumps)
- [x] Inline chat todo card (/todos)
- [x] Side panel todo view (/todos panel)
- [x] Pinned todo band (/todos pin)
- [x] Info widget todo rendering (compact, expanded, widget)
- [x] Todo change diff renderer
- [x] Group support (todo groups with headers)
- [x] Blocked-by dependencies
- [x] Assigned-to field
- [x] Provider input normalization (stringified JSON, etc.)
- [x] Session title derivation from todos
- [x] Clear session todos on reset
