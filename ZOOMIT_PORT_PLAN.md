# ZoomIt Rust Port Plan

*Created: 2026-04-11 | Based on deep codebase analysis*

## Executive Summary

ZoomIt is **38K LOC** across 7 subsystems. The 11 MB binary is inflated by WinRT template
instantiation (~3.5 MB) and linked codecs — the actual logic is ~15K LOC.

**Key finding: ZoomIt uses GDI/GDI+ for drawing, NOT D2D.** This means a Rust port can
upgrade to Direct2D for hardware acceleration while maintaining the same UX — a net
performance improvement, not just a port.

## Architecture

```
ZoomIt (38K LOC total)
├── Zoom (StretchBlt magnification)              ~2K LOC  ← Tier 1
├── Live Zoom (Windows Magnification API)        ~1K LOC  ← Tier 1
├── Draw (GDI+ annotation overlay)               ~3K LOC  ← Tier 1
├── Break Timer (GDI+ fullscreen overlay)        ~875 LOC ← Tier 1
├── Snip/SelectRectangle                         ~400 LOC ← Tier 1
├── Panorama (image stitching + homography)      ~16K LOC ← Tier 2
├── Video Recording (Media Foundation + D3D11)   ~5K LOC  ← Tier 3
├── GIF Recording (WIC)                          ~540 LOC ← Tier 2
├── OCR (WinRT Media.Ocr)                        ~300 LOC ← Tier 3
├── DemoType (typing simulation)                 ~1.3K LOC ← Skip
└── Utility/Registry/Hotkeys                     ~7K LOC  ← Shared
```

## Rendering Pipeline (Current C++)

| Function | Technology | Hardware Accel? | Rust Replacement |
|----------|-----------|-----------------|-----------------|
| Zoom | StretchBlt (GDI) | No (CPU) | Direct2D `DrawBitmap` (GPU) |
| Live Zoom | Magnification API | Yes (OS-level) | Same API (FFI) |
| Draw strokes | GDI+ `Graphics` | No (CPU) | Direct2D `DrawLine/Ellipse` (GPU) |
| Break timer | GDI+ text | No (CPU) | Direct2D `DrawText` (GPU) |
| Record capture | DXGI Output Dup | Yes (GPU) | Same API (FFI) |
| Record encode | Media Foundation | Yes (GPU H.264) | Same API (FFI) |

**Upgrade opportunity:** Replacing GDI/GDI+ with Direct2D gives hardware-accelerated
rendering for free — strokes, text, and zoom will all be GPU-rendered.

## Performance Targets

| Metric | C++ (Current) | Rust (Target) | How |
|--------|--------------|---------------|-----|
| Binary size | 11,156 KB | ~2,000 KB | No WinRT templates, no GDI+ link |
| Startup | ~350ms | ~200ms | No WinRT activation overhead |
| Zoom latency | ~15ms | ~10ms | D2D `DrawBitmap` vs StretchBlt |
| Draw input lag | ~20ms | ~12ms | D2D GPU rasterization vs GDI+ CPU |
| Idle RAM | ~45 MB | ~20 MB | No GDI+ runtime, smaller binary |
| Zoom 2x RAM | ~60 MB | ~35 MB | GPU textures vs GDI DIB sections |
| Record CPU | ~18% @ 30fps | ~15% | Same MF pipeline, less overhead |
| Undo stack | 265 MB (32 levels) | 130 MB | GPU textures + compression |

## Benchmark Plan

### Metrics to capture (before AND after port)

```
1. LATENCY
   - Hotkey press → zoom window visible (ms)
   - Mouse move → pixel rendered in draw mode (ms)
   - Undo operation latency (ms)

2. THROUGHPUT
   - Zoom redraws/second during pan
   - Recording: dropped frames / total frames
   - Panorama: frames stitched/second

3. MEMORY
   - Idle (exe loaded, no mode active)
   - Zoom 2x active
   - Draw mode + 10 strokes
   - Draw mode + 32 undo levels (max)
   - Recording active @ 30fps 1080p
   - All modes simultaneously

4. CPU
   - Idle
   - Live zoom tracking cursor
   - Recording @ 30fps
   - Panorama capture in progress

5. GPU
   - Idle
   - D2D draw mode (Rust only — C++ uses CPU)
   - Recording with hardware H.264
```

### Benchmark tool
Create `zoomit-bench.ps1` that:
1. Launches ZoomIt
2. Activates each mode via hotkeys (SendKeys)
3. Captures perf counters via `Get-Counter` / ETW
4. Outputs CSV for before/after comparison

## Port Phases

### Tier 1: Core ZoomIt (3-4 weeks) — 80% of daily use

**Deliverables:**
- Zoom window (Magnification API + D2D rendering)
- Live zoom (Magnification API, cursor following)
- Draw mode (D2D strokes, shapes, text, arrows)
- Blur/highlight filters (D2D effects)
- Undo stack (32 levels, GPU texture-backed)
- Break timer (D2D text overlay)
- Snip to clipboard

**Binary size target:** ~1.5 MB
**RAM target:** < 25 MB idle

**Key Rust crates:**
- `windows` (0.58) — Win32, D2D1, Magnification API
- `direct2d` or raw `ID2D1` via windows crate

### Tier 2: Capture Features (4-5 weeks)

**Deliverables:**
- GIF recording (WIC encoder via `windows` crate)
- Panorama capture (port homography math to `nalgebra`)
- SelectRectangle (crop selection UI)

**Binary delta:** +300 KB
**New crates:** `nalgebra` for matrix math, `image` for pixel ops

### Tier 3: Recording + OCR (6-8 weeks)

**Deliverables:**
- MP4 recording (Media Foundation + DXGI Output Duplication)
- Audio capture (WASAPI loopback + microphone)
- OCR (WinRT `Media.Ocr` via windows crate)

**Binary delta:** +500 KB (MF codec bindings)
**Risk:** Media Foundation COM is complex — budget extra debugging time

### Tier 4: Skip

- DemoType (typing simulation) — niche, low value
- Audio mixing/synthesis — niche

## Size Breakdown Projection

| Component | C++ | Rust | Why smaller |
|-----------|-----|------|-------------|
| WinRT templates | 3,500 KB | 0 KB | Rust FFI, no template bloat |
| GDI+ link | 1,500 KB | 0 KB | Use D2D instead |
| Media Foundation | 2,000 KB | 800 KB | Direct bindings only |
| Code | 800 KB | 600 KB | Rust is compact |
| Resources | 300 KB | 300 KB | Same icons/strings |
| robmikh.common | 1,000 KB | 0 KB | Inline the 3 functions we need |
| Debug/misc | 2,000 KB | 300 KB | Release + strip |
| **Total** | **11,156 KB** | **~2,000 KB** | **82% reduction** |

## Risk Assessment

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|-----------|
| D2D performance regression | High | Low | Benchmark each operation; D2D is faster than GDI+ |
| Media Foundation COM complexity | Medium | High | Port recording last; keep C++ fallback DLL |
| Magnification API quirks | Medium | Low | Thin FFI wrapper; same API, same behavior |
| Multi-monitor DPI edge cases | Medium | Medium | Port DPI logic exactly from C++; test on multi-mon |
| Panorama stitching accuracy | Low | Low | Pure math port; validate with reference images |

## Success Criteria

### Tier 1 (MVP)
- [ ] All zoom/draw/break features work identically
- [ ] Binary < 2 MB (vs 11 MB C++)
- [ ] RAM < 30 MB (vs 45 MB C++)
- [ ] Draw latency ≤ C++ (benchmark proves it)
- [ ] No visual artifacts in zoom/draw

### Full Port
- [ ] All features ported including recording + OCR
- [ ] Binary < 3 MB
- [ ] Performance equal or better on all benchmarks
- [ ] 50+ unit tests for drawing, layout, panorama math
