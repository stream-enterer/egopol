# Translation Idiom Rosetta Stone

Recurring C++ → Rust translation patterns in this codebase. An LLM comparing widgets should recognize these as equivalent and NOT flag them as bugs.

## Pixel Arithmetic

| C++ Pattern | Rust Equivalent | Notes |
|-------------|-----------------|-------|
| `(x * 257 + 0x8073) >> 16` | `(x * 257 + 0x8073) >> 16` | div255 Blinn formula — must be identical |
| `(x + 128 + ((x + 128) >> 8)) >> 8` | `div255_epi16()` (AVX2) | SIMD-friendly div255 variant — equivalent |
| `alpha * 257` | `alpha * 257` | Scale 0..255 → 0..65535 — must match |
| `(color >> 24) & 0xFF` | `color.r()` or `(self.0 >> 24) & 0xFF` | Channel extraction from packed RGBA |
| `emColor(r, g, b, a)` | `Color::rgba(r, g, b, a)` | Color construction |
| `color.GetRed()` etc. | `color.r()`, `color.g()`, `color.b()`, `color.a()` | Channel access |

## Geometry and Coordinates

| C++ Pattern | Rust Equivalent | Notes |
|-------------|-----------------|-------|
| `double x, y, w, h` | `Rect { x, y, w, h }` (all `f64`) | Logical coordinates |
| `int px, py, pw, ph` | `PixelRect { x, y, w, h }` (all `i32`) | Pixel coordinates |
| `(int)(x + 0.5)` | `x.round() as i32` | Float → pixel rounding |
| `(int)x` | `x as i32` | Float → pixel truncation |
| `emATMatrix` | `AffineMatrix` | 2D affine transform |

## Ownership and Object Model

| C++ Pattern | Rust Equivalent | Notes |
|-------------|-----------------|-------|
| `emPanel* child` | `PanelId` (slotmap key) | Panel references are IDs, not pointers |
| `emPanel& parent` | `PanelId` or `Option<PanelId>` | Parent reference |
| `emModel::Lookup()` | `Context::get::<T>()` | Singleton model lookup |
| `emRef<T>` | `Rc<RefCell<T>>` | Shared ownership |
| `emCrossPtr<T>` | `Weak<RefCell<T>>` | Non-owning back-reference |
| `virtual void Paint()` | `fn paint(&self, ctx: &mut PanelCtx)` | Virtual dispatch → trait method |
| `virtual void Notice()` | `fn notice(&mut self, flags: NoticeFlags, ctx: &mut PanelCtx)` | Notification dispatch |
| `virtual void Input()` | `fn input(&mut self, event: &InputEvent, state: &InputState, ...) -> bool` | Input handling |
| `new emButton(parent, name)` | `tree.add_panel(parent, Button::new())` | Widget construction + tree insertion |
| `delete panel` | `tree.remove_panel(id)` | Panel destruction |

## State and Signals

| C++ Pattern | Rust Equivalent | Notes |
|-------------|-----------------|-------|
| `emSignal SomeSignal` | `SignalId` | Signal identifier |
| `Signal(SomeSignal)` | `ctx.signal(signal_id)` | Fire signal |
| `IsSignaled(sig)` | `ctx.is_signaled(signal_id)` | Check if signal fired |
| `WakeUp()` | `ctx.wake_up()` or engine wake | Engine activation |
| `IsEnabled()` | `state.enabled` | Panel state queries |
| `IsFocused()` | `state.focused` | Focus state |
| `IsInActivePath()` | `state.is_active` | Activation path |
| `IsViewed()` | `state.viewed` | Visibility |
| `GetViewedWidth()` | `state.viewed_width` | Viewport dimensions |

## Layout

| C++ Pattern | Rust Equivalent | Notes |
|-------------|-----------------|-------|
| `emLinearLayout` | `LinearLayout` | 1D weighted layout |
| `emPackLayout` | `PackLayout` | Stacking layout |
| `emRasterLayout` | `RasterLayout` | Grid layout |
| `SetOuterBorderType()` | `with_outer_border(OuterBorderType::X)` | Builder pattern |
| `SetInnerBorderType()` | `with_inner_border(InnerBorderType::X)` | Builder pattern |
| `LayoutChildren(x, y, w, h)` | `fn layout_children(&self, content: Rect, ...) -> Vec<Rect>` | Layout computation |
| `GetBestTallness()` | `fn get_best_tallness() -> f64` | Preferred aspect ratio |

## Widget State Patterns

| C++ Pattern | Rust Equivalent | Notes |
|-------------|-----------------|-------|
| `IsChecked()` | `self.checked` | Boolean state |
| `SetChecked(v)` | `self.checked = v; ctx.signal(...)` | State + signal |
| `GetText()` | `&self.text` | String access |
| `SetText(s)` | `self.set_text(s, ctx)` | String mutation + signal |
| `CheckMouse(x, y, ...)` | Hit test in `input()` with signed-distance | Rounded-rect hit test |
| `DoButton(painter, ...)` | Logic inside `paint()` | Button rendering subroutine |
| `DoLabel(painter, ...)` | Logic inside `paint()` | Label text fitting |
| `HowToText = "..."` | `const HOWTO: &str = "..."` or inline string | Help text constants |

## Error Handling

| C++ Pattern | Rust Equivalent | Notes |
|-------------|-----------------|-------|
| `throw emException(msg)` | `return Err(SomeError::X)` | Exception → Result |
| `try { ... }` | `match result { Ok/Err }` or `?` | Exception handling |
| `assert(condition)` | `assert!(condition)` or `debug_assert!()` | Only for invariants |
| `if (!ptr) return` | `if let Some(x) = opt { ... }` | Null check → Option |

## Constants and Enums

| C++ Pattern | Rust Equivalent | Notes |
|-------------|-----------------|-------|
| `#define FOO 42` | `const FOO: i32 = 42` | Named constant |
| `enum { A, B, C }` | `enum Foo { A, B, C }` | Typed enum |
| `OBT_RECT` | `OuterBorderType::Rect` | Scoped enum variant |
| `IBT_INPUT_FIELD` | `InnerBorderType::InputField` | Scoped enum variant |
| `NF_LAYOUT_CHANGED` | `NoticeFlags::LAYOUT_CHANGED` | Bitflag |
