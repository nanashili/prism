# Startup glass

`glass-prism.png` is the transparent 1254 × 1254 PNG used as a fallback for the first-launch intro when native Liquid Glass is unavailable. It was generated with the built-in imagegen tool using [prompt.txt](prompt.txt). Preserve its alpha channel.

On supported macOS versions, the prism uses [liquid-glass-rs](https://github.com/Stapxs/liquid-glass-rs): a native `NSGlassEffectView` masked to its silhouette above the animated light. That path does not load this image. The window background stays solid during the intro. `src/ui/splash/scene.rs` draws the animated incoming ray and outgoing spectrum in both modes.
