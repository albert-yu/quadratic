use crate::{
    grid::{
        borders::{get_render_horizontal_borders, get_render_vertical_borders},
        js_types::{
            JsHtmlOutput, JsRenderBorder, JsRenderCell, JsRenderCodeCell, JsRenderCodeCellState,
            JsRenderFill,
        },
        CellAlign, CodeCellRunResult, NumericFormatKind,
    },
    CellValue, Pos, Rect,
};

use super::Sheet;

impl Sheet {
    /// checks columns for any column that has data that might render
    pub fn has_render_cells(&self, region: Rect) -> bool {
        self.columns.range(region.x_range()).any(|(_, column)| {
            column.values.has_blocks_in_range(region.y_range())
                || column.spills.has_blocks_in_range(region.y_range())
        })
    }

    /// Returns cell data in a format useful for rendering. This includes only
    /// the data necessary to render raw text values.
    pub fn get_render_cells(&self, region: Rect) -> Vec<JsRenderCell> {
        let columns_iter = region
            .x_range()
            .filter_map(|x| Some((x, self.get_column(x)?)));

        // Fetch ordinary value cells.
        let ordinary_cells = columns_iter.clone().flat_map(|(x, column)| {
            column
                .values
                .values_in_range(region.y_range())
                .map(move |(y, value)| (x, y, column, value, None))
        });

        // Fetch values from code cells.
        let code_output_cells = columns_iter.flat_map(move |(x, column)| {
            column
                .spills
                .blocks_of_range(region.y_range())
                .filter_map(move |block| {
                    let code_cell_pos = self.cell_ref_to_pos(block.content.value)?;
                    let code_cell = self.code_cells.get(&block.content.value)?;

                    let (block_len, cell_error) = if let Some(error) = code_cell.get_error() {
                        (1, Some(CellValue::Error(Box::new(error))))
                    } else {
                        (block.len(), None)
                    };

                    let dx = (x - code_cell_pos.x) as u32;
                    let dy = (block.y - code_cell_pos.y) as u32;

                    Some((0..block_len).filter_map(move |y_within_block| {
                        let y = block.y + y_within_block as i64;
                        let dy = dy + y_within_block as u32;
                        Some((
                            x,
                            y,
                            column,
                            cell_error
                                .clone()
                                .or_else(|| code_cell.get_output_value(dx, dy))?,
                            ((dx, dy) == (0, 0)).then_some(code_cell.language),
                        ))
                    }))
                })
                .flatten()
        });

        itertools::chain(ordinary_cells, code_output_cells)
            .map(|(x, y, column, value, language)| {
                // Only fetch formatting info for non-error, non-HTML, non-blank
                // cells
                let fmt = !matches!(
                    value,
                    CellValue::Blank | CellValue::Error(_) | CellValue::Html(_),
                );
                let mut align = if fmt { column.align.get(y) } else { None };
                let wrap = if fmt { column.wrap.get(y) } else { None };
                let bold = if fmt { column.bold.get(y) } else { None };
                let italic = if fmt { column.italic.get(y) } else { None };
                let text_color = if fmt { column.text_color.get(y) } else { None };

                let value = match value {
                    CellValue::Error(_) => " ERROR".to_string(),
                    CellValue::Html(_) => " HTML".to_string(),
                    CellValue::Number(_) => {
                        // get numeric_format and numeric_decimal to turn number into a string
                        let numeric_format = column.numeric_format.get(y);
                        let is_percentage = numeric_format.as_ref().is_some_and(|numeric_format| {
                            numeric_format.kind == NumericFormatKind::Percentage
                        });
                        let numeric_decimals = self.decimal_places(Pos { x, y }, is_percentage);
                        let numeric_commas = column.numeric_commas.get(y);

                        // if align is not set, set it to right only for numbers
                        align = align.or(Some(CellAlign::Right));

                        value.to_display(numeric_format, numeric_decimals, numeric_commas)
                    }
                    _ => value.to_display(None, None, None),
                };

                JsRenderCell {
                    x,
                    y,

                    value,
                    language,

                    align,
                    wrap,
                    bold,
                    italic,
                    text_color,
                }
            })
            .collect()
    }

    pub fn get_html_output(&self) -> Vec<JsHtmlOutput> {
        self.code_cells
            .iter()
            .filter_map(|(cell_ref, code_cell_value)| {
                let output = code_cell_value.get_output_value(0, 0)?;
                if matches!(output, CellValue::Html(_)) {
                    return None;
                }
                let pos = self.cell_ref_to_pos(*cell_ref)?;
                let output_size = if let Some(render_size) = self.render_size(pos) {
                    (render_size.w, render_size.h)
                } else {
                    (0, 0)
                };
                Some(JsHtmlOutput {
                    sheet_id: self.id.to_string(),
                    x: pos.x,
                    y: pos.y,
                    html: output.to_display(None, None, None),
                    w: output_size.0,
                    h: output_size.1,
                })
            })
            .collect()
    }

    /// Returns all data for rendering cell fill color.
    pub fn get_all_render_fills(&self) -> Vec<JsRenderFill> {
        let mut ret = vec![];
        for (&x, column) in self.columns.iter() {
            for block in column.fill_color.blocks() {
                ret.push(JsRenderFill {
                    x,
                    y: block.y,
                    w: 1,
                    h: block.len() as u32,
                    color: block.content().value.clone(),
                });
            }
        }
        ret
    }
    /// Returns data for rendering cell fill color.
    pub fn get_render_fills(&self, region: Rect) -> Vec<JsRenderFill> {
        let mut ret = vec![];
        for (&x, column) in self.columns.range(region.x_range()) {
            for block in column.fill_color.blocks_covering_range(region.y_range()) {
                ret.push(JsRenderFill {
                    x,
                    y: block.y,
                    w: 1,
                    h: block.len() as u32,
                    color: block.content().value.clone(),
                });
            }
        }
        ret
    }
    /// Returns data for rendering code cells.
    pub fn get_render_code_cells(&self, rect: Rect) -> Vec<JsRenderCodeCell> {
        self.iter_code_cells_locations_in_region(rect)
            .filter_map(|cell_ref| {
                let pos = self.cell_ref_to_pos(cell_ref)?;
                if !rect.contains(pos) {
                    return None;
                }
                let code_cell = self.code_cells.get(&cell_ref)?;
                let output_size = code_cell.output_size();
                let state = match &code_cell.output {
                    Some(output) => match output.result {
                        CodeCellRunResult::Ok { .. } => JsRenderCodeCellState::Success,
                        CodeCellRunResult::Err { .. } => JsRenderCodeCellState::RunError,
                    },
                    None => JsRenderCodeCellState::NotYetRun,
                };
                Some(JsRenderCodeCell {
                    x: pos.x,
                    y: pos.y,
                    w: output_size.w.get(),
                    h: output_size.h.get(),
                    language: code_cell.language,
                    state,
                })
            })
            .collect()
    }

    /// Returns data for all rendering code cells
    pub fn get_all_render_code_cells(&self) -> Vec<JsRenderCodeCell> {
        self.iter_code_cells_locations()
            .filter_map(|cell_ref| {
                let pos = self.cell_ref_to_pos(cell_ref)?;
                let code_cell = self.code_cells.get(&cell_ref)?;
                let output_size = code_cell.output_size();
                Some(JsRenderCodeCell {
                    x: pos.x,
                    y: pos.y,
                    w: output_size.w.get(),
                    h: output_size.h.get(),
                    language: code_cell.language,
                    state: match &code_cell.output {
                        Some(output) => match &output.result {
                            CodeCellRunResult::Ok { .. } => JsRenderCodeCellState::Success,
                            CodeCellRunResult::Err { .. } => JsRenderCodeCellState::RunError,
                        },
                        None => JsRenderCodeCellState::NotYetRun,
                    },
                })
            })
            .collect()
    }

    /// Returns data for rendering horizontal borders.
    pub fn get_render_horizontal_borders(&self) -> Vec<JsRenderBorder> {
        get_render_horizontal_borders(self)
    }

    /// Returns data for rendering vertical borders.
    pub fn get_render_vertical_borders(&self) -> Vec<JsRenderBorder> {
        get_render_vertical_borders(self)
    }
}
