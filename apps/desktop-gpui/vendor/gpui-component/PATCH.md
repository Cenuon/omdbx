Patched from gpui-component 0.9.0.

`calculate_visible_leaf_col_range` now clamps `scroll_x` to the real maximum.
Without it, horizontal rubber-banding pushed the shared scroll offset past the
content edge, the visible column range came back empty, and the header blanked
for a frame. Re-apply when bumping the dependency.

Also added `Input::hide_text(bool)`: paints the editor glyphs transparent so
the SQL filter fields can draw their own monospace, keyword-colored text layer
under the real input (caret and selection stay visible).
