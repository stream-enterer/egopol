# C++ → Rust Port Bug Taxonomy

Checklist of common bug classes when porting C++ to Rust, ordered by frequency/severity in this codebase.

## 1. Integer Arithmetic Divergence

- **Signed/unsigned mismatch**: C++ implicit `int` ↔ `unsigned` promotion. Rust requires explicit casts. Watch for negative intermediate values in blend math being truncated to `u8`/`u32`.
- **Integer overflow semantics**: C++ unsigned overflow wraps silently; Rust panics in debug. The `wrapping_*` methods are needed for intentional wraparound.
- **Division rounding**: C++ integer division truncates toward zero; verify Rust matches (it does, but watch for `as` casts from negative floats).
- **Shift arithmetic**: C++ right-shift of signed is implementation-defined (arithmetic on most platforms). Rust arithmetic right-shift is guaranteed for signed types.
- **Operator precedence**: C++ `&` and `|` have lower precedence than `==`/`!=`. Rust matches, but porting often introduces parenthesization errors.

## 2. Off-by-One and Boundary Errors

- **Loop bounds**: C++ `for (int i = 0; i < n; i++)` vs Rust `0..n` — equivalent, but watch for `<=` → `0..=n` vs `0..n+1`.
- **Clamp boundaries**: C++ `std::clamp(x, lo, hi)` vs Rust `.clamp(lo, hi)` — semantically identical but edge case when `lo > hi` differs (C++ UB, Rust panics).
- **Rect edge inclusion**: C++ `x < x + w` (exclusive right edge) — verify Rust rect methods match this convention.
- **Array indexing**: C++ pointer arithmetic `p + i` vs Rust `&buf[i]` — off-by-one when converting pointer-end to length.

## 3. Missing or Incorrect State Transitions

- **Uninitialized state**: C++ constructors may leave members uninitialized (then set in first use). Rust requires initialization — the chosen default may not match C++ first-use value.
- **Destructor ordering**: C++ destructors run in reverse declaration order. Rust `Drop` runs fields in declaration order. Matters if cleanup has side effects.
- **Boolean flag reset**: C++ may set-and-clear flags in specific sequences. Missing a clear in Rust leaves stale state.
- **Event ordering**: C++ virtual dispatch may call overrides in specific order. Rust trait dispatch is explicit — verify call sequence.

## 4. Float ↔ Integer Conversion Errors

- **Truncation vs rounding**: C++ `(int)x` truncates; Rust `x as i32` also truncates but `x.round() as i32` rounds. Verify which the C++ code intends.
- **Negative float to unsigned**: C++ `(unsigned)(negative_float)` is UB. Rust `negative_float as u32` saturates to 0 in release. Could mask bugs.
- **Float precision in geometry**: C++ `double` and Rust `f64` are identical, but intermediate expression evaluation may differ if C++ uses extended precision (x87). Unlikely on x86-64 SSE but possible.

## 5. Missing Functionality

- **Unported methods**: C++ class may have methods not yet implemented in Rust. Stubs, `todo!()`, or simply absent.
- **Missing virtual overrides**: C++ subclass overrides not translated to Rust trait implementations.
- **Missing signal/callback wiring**: C++ uses virtual dispatch for notifications; Rust uses explicit signal/callback registration — easy to forget to wire.
- **Conditional compilation paths**: C++ `#ifdef` paths that were skipped during port.

## 6. Coordinate System and Transform Errors

- **Pixel vs logical confusion**: Using `f64` where `i32` is needed or vice versa. The codebase enforces `f64` logical, `i32` pixel, `u32` dims.
- **Transform order**: Matrix multiplication order (C++ may use row-major, Rust column-major or vice versa).
- **Clip rect intersection**: Off-by-one in clip rect computation — one-pixel-too-wide or one-pixel-too-narrow.
- **Origin mismatch**: C++ may compute relative to parent; Rust may compute relative to view or panel.

## 7. String and Text Handling

- **Encoding**: C++ `std::string` is byte-oriented; Rust `String` is UTF-8. C++ code may assume single-byte characters.
- **Text measurement**: Font metrics may differ between C++ and Rust implementations.
- **String comparison**: C++ `strcmp` returns int; Rust `==` returns bool. Ordering comparison port errors.

## 8. Memory and Lifetime Patterns

- **Dangling references**: C++ raw pointers to objects that may be deleted. Rust `Weak<T>` upgrade may return `None` where C++ would crash or silently use stale data.
- **Shared mutation**: C++ `this->member = x` in callback. Rust `RefCell` borrow may panic if already borrowed.
- **Collection invalidation**: C++ iterating over vector while modifying it. Rust borrow checker prevents this — the workaround may have different behavior.

## 9. Visual/Rendering Bugs

- **Color channel ordering**: RGBA vs ARGB vs BGRA. The codebase uses RGBA (R[31:24]).
- **Premultiplication**: Alpha-premultiplied vs straight alpha. Mixing them produces wrong blending.
- **Anti-aliasing**: Coverage calculation differences. C++ integer AA vs Rust approximation.
- **Gradient interpolation**: Wrong lookup table size or index calculation.

## 10. Input Handling Bugs

- **Double-click detection**: Timing and distance thresholds must match C++.
- **Modifier key state**: C++ may check modifiers differently than Rust input events.
- **Focus/activation sequencing**: C++ may process focus before or after input; Rust must match the order.
- **Hit testing**: Geometric hit test (e.g., rounded rect signed-distance) must use same formula.
