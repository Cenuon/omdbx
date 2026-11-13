use super::*;

#[derive(Clone)]
pub(crate) struct GridDelegate {
    pub(crate) columns: Vec<String>,
    pub(crate) rows: Vec<Vec<String>>,
    pub(crate) meta: Vec<ColumnMeta>,
    pub(crate) show_comments: bool,
    pub(crate) show_types: bool,
    pub(crate) zoom: f32,
    /// Data-column indexes hidden from the grid.
    pub(crate) hidden: HashSet<usize>,
    /// User-set column widths, normalized to 100% zoom. Shared with the
    /// table event subscriber so resize events can be recorded without
    /// re-entering the table entity.
    pub(crate) widths: Rc<RefCell<HashMap<usize, f32>>>,
    /// Width of the table viewport, measured from layout. Shared so the last
    /// column can absorb the remaining space instead of leaving a blank strip.
    pub(crate) viewport: Rc<Cell<f32>>,
    /// Number of visible data columns frozen from the left.
    pub(crate) frozen: usize,
    pub(crate) row_offset: usize,
    pub(crate) editing: Option<EditingCell>,
    /// Data cells whose value differs from the database.
    pub(crate) dirty: HashSet<(usize, usize)>,
    /// Appended rows that are not in the database yet.
    pub(crate) new_rows: HashSet<usize>,
    /// Existing rows marked for deletion.
    pub(crate) deleted_rows: HashSet<usize>,
    /// Present for grids that expose context menus. `editable` decides whether
    /// the edit actions are enabled.
    pub(crate) weak: Option<gpui_kit::WeakEntity<Omdbx>>,
    /// The owning app, so header actions can open dialogs and update settings.
    pub(crate) app: gpui_kit::WeakEntity<Omdbx>,
    pub(crate) connection: Option<Connection>,
    pub(crate) table_name: Option<String>,
    /// Whether cells/rows can be edited in place.
    pub(crate) editable: bool,
}

impl GridDelegate {
    pub(crate) fn visible_len(&self) -> usize {
        self.columns
            .iter()
            .enumerate()
            .filter(|(index, _)| !self.hidden.contains(index))
            .count()
    }

    /// Map a rendered column index (1-based, after the row-number column) to
    /// its data column.
    pub(crate) fn view_to_data(&self, view: usize) -> Option<usize> {
        if view == 0 {
            return None;
        }
        let mut seen = 0;
        for data in 0..self.columns.len() {
            if self.hidden.contains(&data) {
                continue;
            }
            if seen == view - 1 {
                return Some(data);
            }
            seen += 1;
        }
        None
    }

    /// Map a data column to its rendered column index (1-based).
    pub(crate) fn data_to_view(&self, data: usize) -> Option<usize> {
        if data >= self.columns.len() || self.hidden.contains(&data) {
            return None;
        }
        let mut seen = 0;
        for index in 0..self.columns.len() {
            if index == data {
                return Some(seen + 1);
            }
            if !self.hidden.contains(&index) {
                seen += 1;
            }
        }
        None
    }

    pub(crate) fn has_primary_key(&self) -> bool {
        self.meta.iter().any(|meta| meta.is_primary_key)
    }

    pub(crate) fn base_width(&self, data: usize) -> f32 {
        let mut width = self
            .meta
            .get(data)
            .map(|meta| {
                let text = (meta.name.chars().count() as f32 * 8.)
                    .max(meta.data_type.chars().count() as f32 * 7.)
                    .max(meta.comment.chars().count() as f32 * 11.);
                text + 28.
            })
            .unwrap_or(140.);
        if let Some(meta) = self.meta.get(data) {
            if is_temporal_type(&meta.data_type) {
                // Dates and timestamps must read in full, so size the column
                // from its longest value instead of the column name.
                let longest = self
                    .rows
                    .iter()
                    .filter_map(|row| row.get(data))
                    .map(|value| value.chars().count())
                    .max()
                    .unwrap_or(0);
                width = width.max(longest as f32 * 8. + 28.).max(160.);
            }
        }
        width.clamp(120., 300.)
    }

    pub(crate) fn effective_width(&self, data: usize) -> f32 {
        self.widths
            .borrow()
            .get(&data)
            .copied()
            .unwrap_or_else(|| self.base_width(data))
            * self.zoom
    }

    /// Width of the row-number column: as narrow as the largest visible row
    /// number allows. A fixed 52px column left a wide blank strip against the
    /// sidebar; sizing it to the digits keeps the grid flush to the left.
    pub(crate) fn row_header_width(&self) -> f32 {
        let digits = (self.row_offset + self.rows.len()).max(1).to_string().len();
        (digits as f32 * 8. + 14.).max(30.)
    }

    /// Space left after all visible columns and the row-number column. The
    /// last data column absorbs it so the grid never ends with an empty strip.
    pub(crate) fn stretch_extra(&self) -> f32 {
        let viewport = self.viewport.get();
        if viewport <= 1. || self.visible_len() == 0 {
            return 0.;
        }
        let data_width: f32 = (1..=self.visible_len())
            .filter_map(|view| self.view_to_data(view))
            .map(|data| self.effective_width(data))
            .sum();
        (viewport - self.row_header_width() - data_width - 1.).max(0.)
    }
}

impl TableDelegate for GridDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.visible_len() + 1
    }

    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, index: usize, _: &App) -> Column {
        // right padding only: the cell origin is the column boundary, so the
        // rule can be drawn exactly on it; the header's sort-icon correction
        // is zero because the right padding matches the default
        let padding = gpui_kit::Edges {
            top: px(0.),
            right: px(12.),
            bottom: px(0.),
            left: px(0.),
        };
        if index == 0 {
            return Column::new("#", "#")
                .width(px(self.row_header_width()))
                .fixed_left()
                .resizable(false)
                .movable(false)
                .selectable(false)
                .paddings(padding);
        }
        let Some(data) = self.view_to_data(index) else {
            return Column::new("", "").width(px(0.)).resizable(false);
        };
        let name = self.columns.get(data).cloned().unwrap_or_default();
        let mut width = self.effective_width(data);
        if index == self.visible_len() {
            width += self.stretch_extra();
        }
        let mut column = Column::new(name.clone(), name)
            .width(px(width.clamp(60., 6000.)))
            .min_width(px(40.))
            .resizable(true)
            .movable(false)
            .paddings(padding);
        if index <= self.frozen {
            column = column.fixed_left();
        }
        column
    }

    fn fit_column(
        &mut self,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> Option<gpui_kit::Pixels> {
        let data = self.view_to_data(col_ix)?;
        let zoom = self.zoom;
        let name_width = self
            .columns
            .get(data)
            .map(|name| name.chars().count() as f32 * 8.5)
            .unwrap_or(0.)
            * zoom;
        let meta = self.meta.get(data);
        let type_width = if self.show_types {
            meta.map(|meta| meta.data_type.chars().count() as f32 * 7.)
                .unwrap_or(0.)
                * zoom
        } else {
            0.
        };
        let comment_width = if self.show_comments {
            meta.map(|meta| meta.comment.chars().count() as f32 * 7.5)
                .unwrap_or(0.)
                * zoom
        } else {
            0.
        };
        let value_width = self
            .rows
            .iter()
            .filter_map(|row| row.get(data))
            .map(|value| value.chars().count() as f32 * 7.8 * zoom)
            .fold(0.0, f32::max);
        Some(px(name_width
            .max(type_width)
            .max(comment_width)
            .max(value_width)
            + 8.))
    }

    fn render_tr(
        &mut self,
        row_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        // Stripe only real rows; the component's own stripe option also paints
        // the filler rows below the last record.
        let is_new = self.new_rows.contains(&row_ix);
        let is_deleted = self.deleted_rows.contains(&row_ix);
        div()
            .id(("row", row_ix))
            .when(is_new, |this| this.bg(gpui_kit::rgb(0xFFF0C2)))
            .when(is_deleted, |this| this.bg(gpui_kit::rgb(0xFFE3E3)))
            .when(!is_new && !is_deleted && row_ix % 2 == 1, |this| {
                this.bg(cx.theme().tokens.table_even)
            })
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let muted = cx.theme().colors.muted_foreground;
        let zoom = self.zoom;
        if col_ix == 0 {
            return div()
                .h_full()
                .w_full()
                .flex()
                .items_center()
                .justify_end()
                .text_size(px(11. * zoom))
                .text_color(muted)
                .child("#")
                .into_any_element();
        }
        let Some(data_col) = self.view_to_data(col_ix) else {
            return div().into_any_element();
        };
        let name = self.columns.get(data_col).cloned().unwrap_or_default();
        let (data_type, comment) = self
            .meta
            .get(data_col)
            .map(|meta| (meta.data_type.clone(), meta.comment.clone()))
            .unwrap_or_default();
        let type_color = data_type_color(&data_type, muted);
        let editable = self.editable;
        let has_pk = self.has_primary_key();
        let table = cx.entity();
        let app = self.app.clone();
        div()
            .id(SharedString::from(format!("grid-header-{data_col}")))
            .h_full()
            .w_full()
            .flex()
            .items_center()
            .when(col_ix > 1, |this| {
                this.child(
                    div()
                        .h_full()
                        .w(px(1.))
                        .flex_none()
                        .bg(cx.theme().table_row_border),
                )
                .child(div().w(px(11.)))
            })
            .when(col_ix == 1, |this| this.child(div().w(px(12.))))
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(1.))
                    .font_family(cx.theme().mono_font_family.clone())
                    .child(
                        div()
                            .truncate()
                            .text_size(px(ui::tokens::FONT_HEADER * zoom))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(name.clone()),
                    )
                    .when(self.show_types && !data_type.is_empty(), |this| {
                        this.child(
                            div()
                                .truncate()
                                .text_size(px(ui::tokens::FONT_META * zoom))
                                .text_color(type_color)
                                .child(data_type),
                        )
                    })
                    .when(self.show_comments && !comment.is_empty(), |this| {
                        this.child(
                            div()
                                .truncate()
                                .text_size(px(ui::tokens::FONT_META * zoom))
                                .text_color(gpui_kit::rgb(0x9A9AA0))
                                .child(comment),
                        )
                    }),
            )
            .context_menu(move |menu, window, cx| {
                header_context_menu(
                    menu,
                    window,
                    cx,
                    app.clone(),
                    table.clone(),
                    data_col,
                    name.clone(),
                    editable,
                    has_pk,
                )
            })
            .into_any_element()
    }

    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let muted = cx.theme().colors.muted_foreground;
        let zoom = self.zoom;
        if col == 0 {
            let number = div()
                .h_full()
                .w_full()
                .flex()
                .items_center()
                .justify_end()
                .text_size(px(11. * zoom))
                .text_color(muted)
                .font_family(cx.theme().mono_font_family.clone())
                .font_features(tabular())
                .child(format!("{}", self.row_offset + row + 1));
            if let Some(weak) = self.weak.clone() {
                let table = cx.entity();
                return number
                    .context_menu(move |menu, window, cx| {
                        row_context_menu(menu, window, cx, weak.clone(), table.clone(), row)
                    })
                    .into_any_element();
            }
            return number.into_any_element();
        }
        let Some(data_col) = self.view_to_data(col) else {
            return div().into_any_element();
        };
        let dirty = self.dirty.contains(&(row, data_col));
        if let Some(editing) = &self.editing {
            if editing.row == row && editing.col == data_col {
                let content = match &editing.editor {
                    CellEditor::Text(input) => Input::new(input)
                        .appearance(false)
                        .bordered(false)
                        .w_full()
                        .with_size(gpui_kit::component::Size::XSmall)
                        .text_size(px(ui::tokens::FONT_CELL * zoom))
                        .font_family(cx.theme().mono_font_family.clone())
                        .into_any_element(),
                    CellEditor::Date(state) => DatePicker::new(state)
                        .appearance(false)
                        .with_size(gpui_kit::component::Size::XSmall)
                        .w_full()
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_size(px(ui::tokens::FONT_CELL * zoom))
                        .into_any_element(),
                    CellEditor::Time(state) => TimeField::new(state)
                        .with_size(gpui_kit::component::Size::XSmall)
                        .font_family(cx.theme().mono_font_family.clone())
                        .text_size(px(ui::tokens::FONT_CELL * zoom))
                        .bg(gpui_kit::transparent_black())
                        .border_color(gpui_kit::transparent_black())
                        .into_any_element(),
                };
                return div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(px(ui::tokens::FONT_CELL * zoom))
                    .rounded(px(2.))
                    .border_1()
                    .border_color(cx.theme().colors.foreground)
                    .bg(if dirty {
                        gpui_kit::rgb(0xFFF0C2).into()
                    } else {
                        cx.theme().colors.background
                    })
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(content)
                    .into_any_element();
            }
        }
        let text = self
            .rows
            .get(row)
            .and_then(|cells| cells.get(data_col))
            .cloned()
            .unwrap_or_default();
        let is_null = text == "NULL";
        let numeric = self
            .meta
            .get(data_col)
            .map(|meta| is_numeric_type(&meta.data_type))
            .unwrap_or(false);
        let column_name = self.columns.get(data_col).cloned().unwrap_or_default();
        let cell = div()
            .id(SharedString::from(format!("cell-body-{row}-{col}")))
            .relative()
            .h_full()
            .w_full()
            .flex()
            .items_center()
            .when(dirty, |this| {
                this.child(
                    div()
                        .absolute()
                        .inset_0()
                        .right(px(-12.))
                        .bg(gpui_kit::rgb(0xFFF0C2)),
                )
            })
            .when(col > 1, |this| {
                this.child(
                    div()
                        .h_full()
                        .w(px(1.))
                        .flex_none()
                        .bg(cx.theme().table_row_border),
                )
                .child(div().w(px(11.)))
            })
            .when(col == 1, |this| this.child(div().w(px(12.))))
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .font_family(cx.theme().mono_font_family.clone())
                    .text_size(px(ui::tokens::FONT_CELL * zoom))
                    .when(numeric, |this| this.text_right().font_features(tabular()))
                    .when(is_null, |this| this.text_color(muted))
                    .child(text.clone()),
            );
        if let Some(weak) = self.weak.clone() {
            let table = cx.entity();
            return cell
                .context_menu(move |menu, window, cx| {
                    cell_context_menu(
                        menu,
                        window,
                        cx,
                        weak.clone(),
                        table.clone(),
                        row,
                        data_col,
                        column_name.clone(),
                        text.clone(),
                    )
                })
                .into_any_element();
        }
        cell.into_any_element()
    }

    fn render_last_empty_col(
        &mut self,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        // gpui-component reserves a 12px trailing spacer by default, which
        // reads as an empty column. The last real column already absorbs the
        // remaining width, so this element should not occupy any space.
        div().w(px(0.)).h_full().flex_shrink_0()
    }
}

#[derive(Clone, Copy)]
enum ColumnCopyFormat {
    TsvData,
    TsvNames,
    TsvNamesData,
    CsvData,
    CsvNames,
    CsvNamesData,
    Json,
}

fn column_copy_text(
    delegate: &GridDelegate,
    data_col: usize,
    format: ColumnCopyFormat,
) -> String {
    let name = delegate
        .columns
        .get(data_col)
        .cloned()
        .unwrap_or_default();
    let values: Vec<&str> = delegate
        .rows
        .iter()
        .map(|row| row.get(data_col).map(String::as_str).unwrap_or(""))
        .collect();
    let data = values.join("\n");
    match format {
        ColumnCopyFormat::TsvData => data,
        ColumnCopyFormat::TsvNames => name,
        ColumnCopyFormat::TsvNamesData => {
            if data.is_empty() {
                name
            } else {
                format!("{name}\n{data}")
            }
        }
        ColumnCopyFormat::CsvData => values
            .iter()
            .map(|value| csv_field(value))
            .collect::<Vec<_>>()
            .join("\n"),
        ColumnCopyFormat::CsvNames => csv_field(&name),
        ColumnCopyFormat::CsvNamesData => {
            let mut lines = vec![csv_field(&name)];
            lines.extend(values.iter().map(|value| csv_field(value)));
            lines.join("\n")
        }
        ColumnCopyFormat::Json => {
            let rows: Vec<serde_json::Value> = values
                .iter()
                .map(|value| {
                    let mut object = serde_json::Map::new();
                    object.insert(
                        name.clone(),
                        if *value == "NULL" {
                            serde_json::Value::Null
                        } else {
                            serde_json::Value::String((*value).to_string())
                        },
                    );
                    serde_json::Value::Object(object)
                })
                .collect();
            serde_json::to_string_pretty(&rows).unwrap_or_default()
        }
    }
}

fn column_insert_sql(delegate: &GridDelegate, data_col: usize) -> Result<String, String> {
    let Some(connection) = delegate.connection.as_ref() else {
        return Err("查询结果不支持生成 SQL".to_string());
    };
    let Some(table) = delegate.table_name.as_ref() else {
        return Err("查询结果不支持生成 SQL".to_string());
    };
    let Some(name) = delegate.columns.get(data_col) else {
        return Err("列不存在".to_string());
    };
    let data_type = delegate
        .meta
        .get(data_col)
        .map(|meta| meta.data_type.as_str())
        .unwrap_or_default();
    let mut out = String::new();
    for row in &delegate.rows {
        let value = row.get(data_col).map(String::as_str).unwrap_or_default();
        out.push_str(&format!(
            "INSERT INTO {} ({}) VALUES ({});\n",
            connection.quote(table),
            connection.quote(name),
            sql_literal(value, data_type)
        ));
    }
    if out.is_empty() {
        Err("当前列没有数据".to_string())
    } else {
        Ok(out)
    }
}

fn column_update_sql(delegate: &GridDelegate, data_col: usize) -> Result<String, String> {
    let Some(connection) = delegate.connection.as_ref() else {
        return Err("查询结果不支持生成 SQL".to_string());
    };
    let Some(table) = delegate.table_name.as_ref() else {
        return Err("查询结果不支持生成 SQL".to_string());
    };
    let Some(name) = delegate.columns.get(data_col) else {
        return Err("列不存在".to_string());
    };
    let primary: Vec<usize> = delegate
        .meta
        .iter()
        .enumerate()
        .filter(|(_, meta)| meta.is_primary_key)
        .map(|(index, _)| index)
        .collect();
    if primary.is_empty() {
        return Err("这张表没有主键，无法生成 Update 语句".to_string());
    }
    let data_type = delegate
        .meta
        .get(data_col)
        .map(|meta| meta.data_type.as_str())
        .unwrap_or_default();
    let mut out = String::new();
    for row in &delegate.rows {
        let predicates = primary
            .iter()
            .map(|index| {
                let meta = &delegate.meta[*index];
                let value = row.get(*index).map(String::as_str).unwrap_or("NULL");
                if value == "NULL" {
                    format!("{} IS NULL", connection.quote(&meta.name))
                } else {
                    format!(
                        "{} = {}",
                        connection.quote(&meta.name),
                        sql_literal(value, &meta.data_type)
                    )
                }
            })
            .collect::<Vec<_>>()
            .join(" AND ");
        let value = row.get(data_col).map(String::as_str).unwrap_or_default();
        out.push_str(&format!(
            "UPDATE {} SET {} = {} WHERE {};\n",
            connection.quote(table),
            connection.quote(name),
            sql_literal(value, data_type),
            predicates
        ));
    }
    if out.is_empty() {
        Err("当前列没有数据".to_string())
    } else {
        Ok(out)
    }
}

fn row_copy_text(
    delegate: &GridDelegate,
    row: usize,
    format: ColumnCopyFormat,
) -> String {
    let names = delegate.columns.clone();
    let values = delegate
        .rows
        .get(row)
        .cloned()
        .unwrap_or_default();
    let tsv_data = values
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("\t");
    let tsv_names = names.join("\t");
    let csv_data = values
        .iter()
        .map(|value| csv_field(value))
        .collect::<Vec<_>>()
        .join(",");
    let csv_names = names
        .iter()
        .map(|value| csv_field(value))
        .collect::<Vec<_>>()
        .join(",");
    match format {
        ColumnCopyFormat::TsvData => tsv_data,
        ColumnCopyFormat::TsvNames => tsv_names,
        ColumnCopyFormat::TsvNamesData => format!("{tsv_names}\n{tsv_data}"),
        ColumnCopyFormat::CsvData => csv_data,
        ColumnCopyFormat::CsvNames => csv_names,
        ColumnCopyFormat::CsvNamesData => format!("{csv_names}\n{csv_data}"),
        ColumnCopyFormat::Json => {
            let mut object = serde_json::Map::new();
            for (index, name) in names.iter().enumerate() {
                let value = values.get(index).map(String::as_str).unwrap_or("");
                object.insert(
                    name.clone(),
                    if value == "NULL" {
                        serde_json::Value::Null
                    } else {
                        serde_json::Value::String(value.to_string())
                    },
                );
            }
            serde_json::to_string_pretty(&serde_json::Value::Object(object))
                .unwrap_or_default()
        }
    }
}

fn row_insert_sql(delegate: &GridDelegate, row: usize) -> Result<String, String> {
    let Some(connection) = delegate.connection.as_ref() else {
        return Err("查询结果不支持生成 SQL".to_string());
    };
    let Some(table) = delegate.table_name.as_ref() else {
        return Err("查询结果不支持生成 SQL".to_string());
    };
    let Some(values) = delegate.rows.get(row) else {
        return Err("行不存在".to_string());
    };
    insert_row_sql(connection, table, &delegate.columns, &delegate.meta, values)
        .map(|sql| format!("{sql};"))
}

fn row_update_sql(delegate: &GridDelegate, row: usize) -> Result<String, String> {
    let Some(connection) = delegate.connection.as_ref() else {
        return Err("查询结果不支持生成 SQL".to_string());
    };
    let Some(table) = delegate.table_name.as_ref() else {
        return Err("查询结果不支持生成 SQL".to_string());
    };
    let Some(values) = delegate.rows.get(row) else {
        return Err("行不存在".to_string());
    };
    let primary: Vec<usize> = delegate
        .meta
        .iter()
        .enumerate()
        .filter(|(_, meta)| meta.is_primary_key)
        .map(|(index, _)| index)
        .collect();
    if primary.is_empty() {
        return Err("这张表没有主键，无法生成 Update 语句".to_string());
    }
    let assignments: Vec<(String, String, String)> = delegate
        .meta
        .iter()
        .enumerate()
        .filter(|(_, meta)| !meta.is_primary_key)
        .map(|(index, meta)| {
            (
                meta.name.clone(),
                meta.data_type.clone(),
                values.get(index).cloned().unwrap_or_default(),
            )
        })
        .collect();
    let keys: Vec<(String, String, String)> = primary
        .iter()
        .map(|index| {
            let meta = &delegate.meta[*index];
            (
                meta.name.clone(),
                meta.data_type.clone(),
                values.get(*index).cloned().unwrap_or_default(),
            )
        })
        .collect();
    update_row_sql(connection, table, &assignments, &keys).map(|sql| format!("{sql};"))
}

fn row_context_menu(
    menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
    app: gpui_kit::WeakEntity<Omdbx>,
    table: Entity<TableState<GridDelegate>>,
    row: usize,
) -> PopupMenu {
    let mut menu = app_menu(menu, cx);
    let (zoom, has_pk, editable) = {
        let delegate = table.read(cx).delegate();
        (
            delegate.zoom,
            delegate.meta.iter().any(|meta| meta.is_primary_key),
            delegate.editable,
        )
    };

    let add = app.clone();
    menu = menu.item(
        PopupMenuItem::new("添加记录")
            .disabled(!editable)
            .on_click(move |_, _, cx| {
                let _ = add.update(cx, |this, cx| this.add_row(cx));
            }),
    );

    let remove = app.clone();
    menu = menu.item(
        PopupMenuItem::new("删除记录")
            .disabled(!editable || !has_pk)
            .on_click(move |_, _, cx| {
                let _ = remove.update(cx, |this, cx| this.delete_row_at(row, cx));
            }),
    );

    menu = menu.separator();

    let copy_table = table.clone();
    menu = menu.item(PopupMenuItem::new("复制").on_click(move |_, _, cx| {
        let text = row_copy_text(copy_table.read(cx).delegate(), row, ColumnCopyFormat::TsvData);
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
    }));

    let names_table = table.clone();
    menu = menu.item(PopupMenuItem::new("复制字段名称").on_click(move |_, _, cx| {
        let names = names_table.read(cx).delegate().columns.join("\t");
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(names));
    }));

    let submenu_table = table.clone();
    let submenu_app = app.clone();
    menu = menu.submenu("复制为", window, cx, move |menu, window, cx| {
        let mut menu = app_menu(menu, cx);

        let insert_table = submenu_table.clone();
        let insert_app = submenu_app.clone();
        menu = menu.item(
            PopupMenuItem::new("Insert 语句").on_click(move |_, _, cx| {
                let result = row_insert_sql(insert_table.read(cx).delegate(), row);
                match result {
                    Ok(sql) => {
                        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(sql))
                    }
                    Err(message) => {
                        let _ = insert_app
                            .update(cx, |this, cx| this.show_notice(&message, cx));
                    }
                }
            }),
        );

        let update_table = submenu_table.clone();
        let update_app = submenu_app.clone();
        menu = menu.item(
            PopupMenuItem::new("Update 语句")
                .disabled(!has_pk)
                .on_click(move |_, _, cx| {
                    let result = row_update_sql(update_table.read(cx).delegate(), row);
                    match result {
                        Ok(sql) => cx.write_to_clipboard(
                            gpui_kit::ClipboardItem::new_string(sql),
                        ),
                        Err(message) => {
                            let _ = update_app
                                .update(cx, |this, cx| this.show_notice(&message, cx));
                        }
                    }
                }),
        );

        menu = menu.separator();
        for (label, format) in [
            ("制表符分隔值（数据）", ColumnCopyFormat::TsvData),
            ("制表符分隔值（字段名称）", ColumnCopyFormat::TsvNames),
            (
                "制表符分隔值（字段名称和数据）",
                ColumnCopyFormat::TsvNamesData,
            ),
        ] {
            let copy_table = submenu_table.clone();
            menu = menu.item(PopupMenuItem::new(label).on_click(move |_, _, cx| {
                let text = row_copy_text(copy_table.read(cx).delegate(), row, format);
                cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
            }));
        }

        menu = menu.separator();
        for (label, format) in [
            ("逗号分隔值（数据）", ColumnCopyFormat::CsvData),
            ("逗号分隔值（字段名称）", ColumnCopyFormat::CsvNames),
            (
                "逗号分隔值（字段名称和数据）",
                ColumnCopyFormat::CsvNamesData,
            ),
        ] {
            let copy_table = submenu_table.clone();
            menu = menu.item(PopupMenuItem::new(label).on_click(move |_, _, cx| {
                let text = row_copy_text(copy_table.read(cx).delegate(), row, format);
                cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
            }));
        }

        menu = menu.separator();
        let json_table = submenu_table.clone();
        menu = menu.item(PopupMenuItem::new("JSON").on_click(move |_, _, cx| {
            let text = row_copy_text(
                json_table.read(cx).delegate(),
                row,
                ColumnCopyFormat::Json,
            );
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
        }));

        let _ = window;
        menu
    });

    let paste_app = app.clone();
    menu = menu.item(
        PopupMenuItem::new("粘贴")
            .disabled(!editable)
            .on_click(move |_, _, cx| {
                let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
                    return;
                };
                let first = text.lines().next().unwrap_or_default();
                let values: Vec<String> =
                    first.split('\t').map(|value| value.to_string()).collect();
                let _ = paste_app.update(cx, |this, cx| {
                    this.paste_row(row, &values, cx)
                });
            }),
    );

    menu = menu.separator();

    let height_app = app.clone();
    menu = menu.item(
        PopupMenuItem::new("设置行高...").on_click(move |_, window, cx| {
            let _ = height_app.update(cx, |this, cx| {
                this.open_width_dialog(WidthTarget::RowHeight, window, cx)
            });
        }),
    );

    let zoom_table = table.clone();
    let zoom_app = app.clone();
    menu.submenu("缩放", window, cx, move |menu, _window, cx| {
        let mut menu = app_menu(menu, cx);
        for &(label, value) in GRID_ZOOM_LEVELS {
            let table = zoom_table.clone();
            let app = zoom_app.clone();
            menu = menu.item(
                PopupMenuItem::new(label)
                    .checked((zoom - value).abs() < 0.001)
                    .on_click(move |_, _, cx| {
                        table.update(cx, |table, cx| {
                            table.delegate_mut().zoom = value;
                            table.refresh(cx);
                            cx.notify();
                        });
                        let _ = app.update(cx, |this, cx| {
                            this.grid_zoom = value;
                            cx.notify();
                        });
                    }),
            );
        }
        menu
    })
}

fn header_context_menu(
    menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
    app: gpui_kit::WeakEntity<Omdbx>,
    table: Entity<TableState<GridDelegate>>,
    data_col: usize,
    column_name: String,
    editable: bool,
    has_pk: bool,
) -> PopupMenu {
    let mut menu = app_menu(menu, cx);
    let (show_types, show_comments, zoom, frozen, view_pos) = {
        let delegate = table.read(cx).delegate();
        (
            delegate.show_types,
            delegate.show_comments,
            delegate.zoom,
            delegate.frozen,
            delegate.data_to_view(data_col),
        )
    };

    let copy_table = table.clone();
    menu = menu.item(PopupMenuItem::new("复制").on_click(move |_, _, cx| {
        let text = column_copy_text(
            copy_table.read(cx).delegate(),
            data_col,
            ColumnCopyFormat::TsvData,
        );
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
    }));

    let name = column_name.clone();
    menu = menu.item(PopupMenuItem::new("复制字段名称").on_click(move |_, _, cx| {
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(name.clone()));
    }));

    let submenu_table = table.clone();
    let submenu_app = app.clone();
    menu = menu.submenu("复制为", window, cx, move |menu, window, cx| {
        let mut menu = app_menu(menu, cx);

        let insert_table = submenu_table.clone();
        let insert_app = submenu_app.clone();
        menu = menu.item(
            PopupMenuItem::new("Insert 语句")
                .disabled(!editable)
                .on_click(move |_, _, cx| {
                    let result = column_insert_sql(insert_table.read(cx).delegate(), data_col);
                    match result {
                        Ok(sql) => cx
                            .write_to_clipboard(gpui_kit::ClipboardItem::new_string(sql)),
                        Err(message) => {
                            let _ = insert_app
                                .update(cx, |this, cx| this.show_notice(&message, cx));
                        }
                    }
                }),
        );

        let update_table = submenu_table.clone();
        let update_app = submenu_app.clone();
        menu = menu.item(
            PopupMenuItem::new("Update 语句")
                .disabled(!editable || !has_pk)
                .on_click(move |_, _, cx| {
                    let result = column_update_sql(update_table.read(cx).delegate(), data_col);
                    match result {
                        Ok(sql) => cx
                            .write_to_clipboard(gpui_kit::ClipboardItem::new_string(sql)),
                        Err(message) => {
                            let _ = update_app
                                .update(cx, |this, cx| this.show_notice(&message, cx));
                        }
                    }
                }),
        );

        menu = menu.separator();
        for (label, format) in [
            ("制表符分隔值（数据）", ColumnCopyFormat::TsvData),
            ("制表符分隔值（字段名称）", ColumnCopyFormat::TsvNames),
            (
                "制表符分隔值（字段名称和数据）",
                ColumnCopyFormat::TsvNamesData,
            ),
        ] {
            let copy_table = submenu_table.clone();
            menu = menu.item(PopupMenuItem::new(label).on_click(move |_, _, cx| {
                let text = column_copy_text(copy_table.read(cx).delegate(), data_col, format);
                cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
            }));
        }

        menu = menu.separator();
        for (label, format) in [
            ("逗号分隔值（数据）", ColumnCopyFormat::CsvData),
            ("逗号分隔值（字段名称）", ColumnCopyFormat::CsvNames),
            (
                "逗号分隔值（字段名称和数据）",
                ColumnCopyFormat::CsvNamesData,
            ),
        ] {
            let copy_table = submenu_table.clone();
            menu = menu.item(PopupMenuItem::new(label).on_click(move |_, _, cx| {
                let text = column_copy_text(copy_table.read(cx).delegate(), data_col, format);
                cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
            }));
        }

        menu = menu.separator();
        let json_table = submenu_table.clone();
        menu = menu.item(PopupMenuItem::new("JSON").on_click(move |_, _, cx| {
            let text = column_copy_text(
                json_table.read(cx).delegate(),
                data_col,
                ColumnCopyFormat::Json,
            );
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(text));
        }));

        let _ = window;
        let _ = cx;
        menu
    });

    menu = menu.separator();

    let paste_app = app.clone();
    let paste_table = table.clone();
    menu = menu.item(
        PopupMenuItem::new("粘贴")
            .disabled(!editable)
            .on_click(move |_, _, cx| {
                let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
                    return;
                };
                let start = paste_table
                    .read(cx)
                    .selected_cell()
                    .map(|(row, _)| row)
                    .or_else(|| paste_table.read(cx).selected_row())
                    .unwrap_or(0);
                let _ = paste_app.update(cx, |this, cx| {
                    let mut count = 0;
                    for (offset, line) in text.lines().enumerate() {
                        let row = start + offset;
                        if row >= this.rows.len() {
                            break;
                        }
                        this.stage_cell_value(row, data_col, line, cx);
                        count += 1;
                    }
                    if count > 0 {
                        this.show_notice(&format!("已粘贴 {count} 行"), cx);
                    }
                });
            }),
    );

    menu = menu.separator();

    let dialog_app = app.clone();
    menu = menu.item(
        PopupMenuItem::new("显示或隐藏列...").on_click(move |_, _, cx| {
            let _ = dialog_app.update(cx, |this, cx| {
                this.column_vis_open = true;
                cx.notify();
            });
        }),
    );

    let hide_app = app.clone();
    menu = menu.item(PopupMenuItem::new("隐藏列").on_click(move |_, window, cx| {
        let _ = hide_app.update(cx, |this, cx| {
            this.toggle_column_hidden(data_col, true, window, cx);
        });
    }));

    let freeze_table = table.clone();
    let is_frozen = view_pos.map(|pos| pos <= frozen).unwrap_or(false);
    menu = menu.item(
        PopupMenuItem::new("冻结列")
            .checked(is_frozen)
            .on_click(move |_, _, cx| {
                freeze_table.update(cx, |table, cx| {
                    let boundary = table.delegate().data_to_view(data_col);
                    let current = table.delegate().frozen;
                    if let Some(boundary) = boundary {
                        let next = if current == boundary { 0 } else { boundary };
                        table.delegate_mut().frozen = next;
                        table.refresh(cx);
                        cx.notify();
                    }
                });
            }),
    );

    menu = menu.separator();

    let width_app = app.clone();
    menu = menu.item(
        PopupMenuItem::new("设置列宽...").on_click(move |_, window, cx| {
            let _ = width_app.update(cx, |this, cx| {
                this.open_width_dialog(WidthTarget::Column(data_col), window, cx)
            });
        }),
    );

    let all_width_app = app.clone();
    menu = menu.item(
        PopupMenuItem::new("设置所有列宽度...").on_click(move |_, window, cx| {
            let _ = all_width_app.update(cx, |this, cx| {
                this.open_width_dialog(WidthTarget::All, window, cx)
            });
        }),
    );

    let fit_table = table.clone();
    menu = menu.item(
        PopupMenuItem::new("调整列宽以适应内容").on_click(move |_, window, cx| {
            fit_table.update(cx, |table, cx| {
                if let Some(view) = table.delegate().data_to_view(data_col) {
                    table.fit_col(view, window, cx);
                }
            });
        }),
    );

    let fit_all_table = table.clone();
    menu = menu.item(
        PopupMenuItem::new("调整所有列的适合大小").on_click(move |_, window, cx| {
            fit_all_table.update(cx, |table, cx| {
                let count = table.delegate().visible_len();
                for view in 1..=count {
                    table.fit_col(view, window, cx);
                }
            });
        }),
    );

    menu = menu.separator();

    let types_table = table.clone();
    let types_app = app.clone();
    menu = menu.item(
        PopupMenuItem::new("显示字段类型")
            .checked(show_types)
            .on_click(move |_, _, cx| {
                let mut next = false;
                types_table.update(cx, |table, cx| {
                    {
                        let delegate = table.delegate_mut();
                        delegate.show_types = !delegate.show_types;
                        next = delegate.show_types;
                    }
                    table.refresh(cx);
                    cx.notify();
                });
                let _ = types_app.update(cx, |this, cx| {
                    this.grid_show_types = next;
                    cx.notify();
                });
            }),
    );

    let comments_table = table.clone();
    let comments_app = app.clone();
    menu = menu.item(
        PopupMenuItem::new("显示注释")
            .checked(show_comments)
            .on_click(move |_, _, cx| {
                let mut next = false;
                comments_table.update(cx, |table, cx| {
                    {
                        let delegate = table.delegate_mut();
                        delegate.show_comments = !delegate.show_comments;
                        next = delegate.show_comments;
                    }
                    table.refresh(cx);
                    cx.notify();
                });
                let _ = comments_app.update(cx, |this, cx| {
                    this.show_comments = next;
                    cx.notify();
                });
            }),
    );

    menu = menu.separator();

    let zoom_table = table.clone();
    let zoom_app = app.clone();
    menu = menu.submenu("缩放", window, cx, move |menu, _window, cx| {
        let mut menu = app_menu(menu, cx);
        for &(label, value) in GRID_ZOOM_LEVELS {
            let table = zoom_table.clone();
            let app = zoom_app.clone();
            menu = menu.item(
                PopupMenuItem::new(label)
                    .checked((zoom - value).abs() < 0.001)
                    .on_click(move |_, _, cx| {
                        table.update(cx, |table, cx| {
                            table.delegate_mut().zoom = value;
                            table.refresh(cx);
                            cx.notify();
                        });
                        let _ = app.update(cx, |this, cx| {
                            this.grid_zoom = value;
                            cx.notify();
                        });
                    }),
            );
        }
        menu
    });

    menu
}

fn cell_context_menu(
    menu: PopupMenu,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
    weak: gpui_kit::WeakEntity<Omdbx>,
    table: Entity<TableState<GridDelegate>>,
    row: usize,
    col: usize,
    column: String,
    value: String,
) -> PopupMenu {
    let mut menu = app_menu(menu, cx);
    let editable = table.read(cx).delegate().editable;

    let setter = weak.clone();
    menu = menu.item(
        PopupMenuItem::new("设置为空白字符串")
            .disabled(!editable)
            .on_click(move |_, _, cx| {
            let _ = setter.update(cx, |this, cx| {
                this.stage_cell_value(row, col, "", cx)
            });
        }),
    );
    let setter = weak.clone();
    menu = menu.item(
        PopupMenuItem::new("设置为 NULL")
            .disabled(!editable)
            .on_click(move |_, _, cx| {
                let _ = setter.update(cx, |this, cx| {
                    this.stage_cell_value(row, col, "NULL", cx)
                });
            }),
    );

    let uuid_weak = weak.clone();
    menu = menu.submenu("生成 UUID", window, cx, move |menu, _, _cx| {
        let v4 = uuid_weak.clone();
        let v7 = uuid_weak.clone();
        menu.item(PopupMenuItem::new("UUID v4").on_click(move |_, _, cx| {
            let value = uuid::Uuid::new_v4().to_string();
            let _ = v4.update(cx, |this, cx| {
                this.stage_cell_value(row, col, &value, cx)
            });
        }))
        .item(PopupMenuItem::new("UUID v7").on_click(move |_, _, cx| {
            let value = uuid::Uuid::now_v7().to_string();
            let _ = v7.update(cx, |this, cx| {
                this.stage_cell_value(row, col, &value, cx)
            });
        }))
    });

    let edit = weak.clone();
    let edit_table = table.clone();
    menu = menu.item(
        PopupMenuItem::new("在单元格编辑器中编辑")
            .disabled(!editable)
            .on_click(move |_, window, cx| {
            let Some(view) = edit_table.read(cx).delegate().data_to_view(col) else {
                return;
            };
            let _ = edit.update(cx, |this, cx| {
                this.begin_cell_edit(row, view, &edit_table, window, cx)
            });
        }),
    );

    let add = weak.clone();
    let remove = weak.clone();
    menu = menu
        .separator()
        .item(
            PopupMenuItem::new("添加记录")
                .disabled(!editable)
                .on_click(move |_, _, cx| {
                    let _ = add.update(cx, |this, cx| this.add_row(cx));
                }),
        )
        .item(
            PopupMenuItem::new("删除记录...")
                .disabled(!editable)
                .on_click(move |_, _, cx| {
                    let _ = remove.update(cx, |this, cx| this.delete_row_at(row, cx));
                }),
        );

    let copy = value.clone();
    let copy_name = column.clone();
    menu = menu
        .separator()
        .item(PopupMenuItem::new("复制").on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(copy.clone()));
        }))
        .item(
            PopupMenuItem::new("复制字段名称").on_click(move |_, _, cx| {
                cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                    copy_name.clone(),
                ));
            }),
        );

    let copy_value = value.clone();
    menu = menu.submenu("复制为", window, cx, move |menu, _, _cx| {
        let csv = csv_field(&copy_value);
        let json = serde_json::to_string(&copy_value).unwrap_or_default();
        let sql = format!("'{}'", copy_value.replace('\'', "''"));
        menu.item(PopupMenuItem::new("CSV").on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(csv.clone()));
        }))
        .item(PopupMenuItem::new("JSON").on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(
                json.clone(),
            ));
        }))
        .item(PopupMenuItem::new("SQL").on_click(move |_, _, cx| {
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(sql.clone()));
        }))
    });

    let paste = weak.clone();
    menu = menu.item(
        PopupMenuItem::new("粘贴")
            .disabled(!editable)
            .on_click(move |_, _, cx| {
                let text = cx
                    .read_from_clipboard()
                    .and_then(|item| item.text());
        match text {
            Some(text) => {
                let _ = paste.update(cx, |this, cx| {
                    this.stage_cell_value(row, col, &text, cx)
                });
            }
            None => {
                let _ =
                    paste.update(cx, |this, cx| this.show_notice("剪贴板没有文本", cx));
            }
        }
    }));

    let save = weak.clone();
    menu = menu.item(
        PopupMenuItem::new("保存数据为...").on_click(move |_, _, cx| {
            let _ = save.update(cx, |this, cx| {
                this.export_table(ExportFormat::Csv, true, cx)
            });
        }),
    );

    menu = menu.separator();
    let toggle = weak.clone();
    menu = menu.item(
        PopupMenuItem::new("显示/隐藏筛选&排序").on_click(move |_, _, cx| {
            let _ = toggle.update(cx, |this, cx| {
                this.filters_visible = !this.filters_visible;
                cx.notify();
            });
        }),
    );

    let filter = weak.clone();
    let clear_filter = weak.clone();
    menu = menu.submenu("筛选", window, cx, move |menu, _, _cx| {
        let by_value = filter.clone();
        let clear = clear_filter.clone();
        menu.item(
            PopupMenuItem::new("按此值筛选").on_click(move |_, window, cx| {
                let _ = by_value.update(cx, |this, cx| {
                    this.filter_by_cell(row, col, window, cx)
                });
            }),
        )
        .item(PopupMenuItem::new("清空筛选").on_click(move |_, window, cx| {
            let _ = clear.update(cx, |this, cx| this.clear_where(window, cx));
        }))
    });

    let sort = weak.clone();
    menu = menu.submenu("排序", window, cx, move |menu, _, _cx| {
        let asc = sort.clone();
        let desc = sort.clone();
        menu.item(PopupMenuItem::new("升序").on_click(move |_, window, cx| {
            let _ = asc.update(cx, |this, cx| this.sort_by_cell(col, true, window, cx));
        }))
        .item(PopupMenuItem::new("降序").on_click(move |_, window, cx| {
            let _ =
                desc.update(cx, |this, cx| this.sort_by_cell(col, false, window, cx));
        }))
    });

    let clear_all = weak.clone();
    menu = menu.item(
        PopupMenuItem::new("移除所有筛选 & 排序").on_click(move |_, window, cx| {
            let _ =
                clear_all.update(cx, |this, cx| this.remove_filters(window, cx));
        }),
    );

    let zoom_app = weak.clone();
    let zoom_table = table.clone();
    menu = menu.separator().submenu("缩放", window, cx, move |menu, _, cx| {
        let mut menu = app_menu(menu, cx);
        for &(label, value) in GRID_ZOOM_LEVELS {
            let app = zoom_app.clone();
            let table = zoom_table.clone();
            menu = menu.item(
                PopupMenuItem::new(label).on_click(move |_, _, cx| {
                table.update(cx, |table, cx| {
                    table.delegate_mut().zoom = value;
                    table.refresh(cx);
                    cx.notify();
                });
                let _ = app.update(cx, |this, cx| {
                    this.grid_zoom = value;
                    cx.notify();
                });
            }));
        }
        menu
    });

    let refresh = weak.clone();
    menu.item(PopupMenuItem::new("刷新").on_click(move |_, window, cx| {
        let _ = refresh.update(cx, |this, cx| this.refresh_active(window, cx));
    }))
}
