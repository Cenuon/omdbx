use crate::*;

impl Omdbx {
    pub(crate) fn suggestions_for(&self, value: &str) -> Vec<Suggestion> {
        let word = last_word(value);
        if word.is_empty() {
            return Vec::new();
        }
        let needle = word.to_lowercase();
        // Lower rank wins: field prefix, keyword prefix, field substring,
        // keyword substring.
        let mut ranked: Vec<(usize, String, Suggestion)> = Vec::new();
        for (index, column) in self.columns.iter().enumerate() {
            let lower = column.to_lowercase();
            let Some(position) = lower.find(&needle) else {
                continue;
            };
            let rank = if position == 0 { 0 } else { 2 };
            let detail = self
                .column_meta
                .get(index)
                .map(|meta| {
                    if meta.comment.is_empty() {
                        meta.data_type.clone()
                    } else {
                        meta.comment.clone()
                    }
                })
                .unwrap_or_default();
            ranked.push((
                rank,
                lower,
                Suggestion {
                    text: column.clone(),
                    detail,
                },
            ));
        }
        for keyword in SQL_KEYWORDS {
            let lower = keyword.to_lowercase();
            let Some(position) = lower.find(&needle) else {
                continue;
            };
            let rank = if position == 0 { 1 } else { 3 };
            ranked.push((
                rank,
                lower,
                Suggestion {
                    text: keyword.to_string(),
                    detail: "SQL".to_string(),
                },
            ));
        }
        ranked.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.text.cmp(&b.2.text)));
        ranked
            .into_iter()
            .take(8)
            .map(|(_, _, suggestion)| suggestion)
            .collect()
    }

    pub(crate) fn suggestion_selection(&self, field: SuggestField) -> usize {
        match field {
            SuggestField::Where => self.where_selected,
            SuggestField::Order => self.order_selected,
        }
    }

    pub(crate) fn set_suggestion_selection(&mut self, field: SuggestField, index: usize) {
        match field {
            SuggestField::Where => self.where_selected = index,
            SuggestField::Order => self.order_selected = index,
        }
    }

    pub(crate) fn close_suggestions(&mut self, field: SuggestField) {
        match field {
            SuggestField::Where => self.where_open = false,
            SuggestField::Order => self.order_open = false,
        }
    }

    pub(crate) fn pick_suggestion(
        &mut self,
        field: SuggestField,
        column: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (state, value) = match field {
            SuggestField::Where => (self.table_where.clone(), self.where_value.clone()),
            SuggestField::Order => (self.table_order.clone(), self.order_value.clone()),
        };
        let word = last_word(&value);
        let base = value.strip_suffix(&word).unwrap_or(&value).to_string();
        match field {
            SuggestField::Where => self.where_suppress_open = true,
            SuggestField::Order => self.order_suppress_open = true,
        }
        let completed = format!("{base}{column}");
        state.update(cx, |state, cx| state.set_value(completed.clone(), window, cx));
        // set_value does not emit Change for every input mode; keep the
        // painted layer in step with the real value.
        match field {
            SuggestField::Where => self.where_value = completed,
            SuggestField::Order => self.order_value = completed,
        }
        match field {
            SuggestField::Where => {
                self.where_suppress_open = false;
                self.where_open = false;
            }
            SuggestField::Order => {
                self.order_suppress_open = false;
                self.order_open = false;
            }
        }
        cx.notify();
    }

    pub(crate) fn handle_suggestion_key(
        &mut self,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let where_suggestions = if self.where_open {
            self.suggestions_for(&self.where_value)
        } else {
            Vec::new()
        };
        let order_suggestions = if self.order_open {
            self.suggestions_for(&self.order_value)
        } else {
            Vec::new()
        };
        let (field, suggestions) = if !where_suggestions.is_empty() {
            (SuggestField::Where, where_suggestions)
        } else if !order_suggestions.is_empty() {
            (SuggestField::Order, order_suggestions)
        } else {
            if key == "enter" {
                self.load_active(window, cx);
                return true;
            }
            return false;
        };
        let selected = self
            .suggestion_selection(field)
            .min(suggestions.len() - 1);
        match key {
            "up" => {
                self.set_suggestion_selection(field, selected.saturating_sub(1));
                cx.notify();
                true
            }
            "down" => {
                self.set_suggestion_selection(field, (selected + 1).min(suggestions.len() - 1));
                cx.notify();
                true
            }
            "enter" | "tab" => {
                let column = suggestions[selected].text.clone();
                self.pick_suggestion(field, &column, window, cx);
                true
            }
            "escape" => {
                self.close_suggestions(field);
                cx.notify();
                true
            }
            _ => false,
        }
    }

    pub(crate) fn suggest_menu(
        &self,
        field: SuggestField,
        cx: &mut Context<Self>,
    ) -> Option<gpui_kit::AnyElement> {
        let (open, value, selected) = match field {
            SuggestField::Where => (self.where_open, self.where_value.as_str(), self.where_selected),
            SuggestField::Order => (self.order_open, self.order_value.as_str(), self.order_selected),
        };
        if !open {
            return None;
        }
        let suggestions = self.suggestions_for(value);
        if suggestions.is_empty() {
            return None;
        }
        let selected = selected.min(suggestions.len() - 1);
        let colors = cx.theme().colors;
        let muted = colors.muted_foreground;
        let accent: Hsla = match field {
            SuggestField::Where => gpui_kit::rgb(0x2F6BFF).into(),
            SuggestField::Order => gpui_kit::rgb(0xE8722C).into(),
        };
        let prefix = match field {
            SuggestField::Where => "where",
            SuggestField::Order => "order",
        };
        let needle = last_word(value).to_lowercase();

        Some(
            deferred(
                v_flex()
                    .absolute()
                    .occlude()
                    .top(px(28.))
                    .left_0()
                    .right_0()
                    .p_1()
                    .gap(px(1.))
                    .rounded_md()
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.background)
                    .shadow_md()
                    .children(suggestions.into_iter().enumerate().map(
                        |(index, suggestion)| {
                            let text = suggestion.text.clone();
                            let match_at = text.to_lowercase().find(&needle);
                            let name: gpui_kit::AnyElement = match match_at {
                                Some(start) if !needle.is_empty() => {
                                    let end = start + needle.len();
                                    h_flex()
                                        .flex_none()
                                        .child(text[..start].to_string())
                                        .child(
                                            div()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(accent)
                                                .child(text[start..end].to_string()),
                                        )
                                        .child(text[end..].to_string())
                                        .into_any_element()
                                }
                                _ => div().child(text.clone()).into_any_element(),
                            };
                            let insert = text;
                            h_flex()
                                .id(SharedString::from(format!("{prefix}-{insert}")))
                                .h(px(26.))
                                .px_2()
                                .gap_2()
                                .items_center()
                                .rounded_md()
                                .text_size(px(12.))
                                .when(index == selected, |this| this.bg(colors.sidebar_accent))
                                .hover(|style| style.bg(colors.sidebar_accent))
                                .child(name)
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(11.))
                                        .text_color(muted)
                                        .child(suggestion.detail),
                                )
                                .child(
                                    div()
                                        .flex_none()
                                        .h(px(16.))
                                        .px_1()
                                        .flex()
                                        .items_center()
                                        .rounded(px(3.))
                                        .border_1()
                                        .border_color(colors.border)
                                        .text_size(px(9.))
                                        .text_color(muted)
                                        .child("Tab"),
                                )
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |this, _, window, cx| {
                                        this.pick_suggestion(field, &insert, window, cx);
                                    }),
                                )
                        },
                    )),
            )
            .into_any_element(),
        )
    }

}
