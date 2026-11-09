Patched from gpui-component 0.9.0.

`calculate_visible_leaf_col_range` now clamps `scroll_x` to the real maximum.
Without it, horizontal rubber-banding pushed the shared scroll offset past the
content edge, the visible column range came back empty, and the header blanked
for a frame. Re-apply when bumping the dependency.
