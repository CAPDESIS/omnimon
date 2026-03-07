# UI/UX & AI Overhaul Architecture

## 1. UI/UX, Micro-animations, and Version UI
- **Framework:** Migration/Update to Svelte 5.
- **Theming:** Introduce customizable themes (Dark, Light, System, High Contrast).
- **Micro-animations:** Implement smooth transitions for charts (Network, Energy) and data tables using Svelte 5's animation APIs.
- **Native Icons:** Render exact native application icons to improve visual identification.
- **Version Visibility:** Software version clearly displayed in the main header or footer to quickly identify the running build.

## 2. GitHub Sponsors
- Integrate a modern Sponsor button/banner pointing to:
  [https://github.com/sponsors/chochy2001/dashboard](https://github.com/sponsors/chochy2001/dashboard)

## 3. AI Autoconfiguration & Tool Calling
- **Objective:** Allow users to say "I want to configure A, B, and C" and the AI will update system settings automatically.
- **Implementation:** Introduce a Tool Calling layer in the AI Service that maps user intent to configuration update functions.

## 4. AI Context & Data Optimization
- **Data Pruning:** Restructure the raw process/telemetry data sent to the AI. Only send the top consumers (CPU, Memory, Energy) and active network connections.
- **Batching:** Send data in batches rather than continuous streams to reduce token usage and context window saturation.

## 5. Natural Language Explanations
- The AI will provide explanations for alerts tailored to non-technical users.
- *Example:* Instead of showing high `SIGSEGV` or `rx_bytes/s` errors, translate it to "This process has been downloading a lot of data recently, it seems to be updating or syncing files."

## 6. User Profiles
- Overhaul the User Profiles configuration UI.
- Add features to switch between profiles easily, each holding its own specific threshold settings, themes, and AI prompt preferences.
