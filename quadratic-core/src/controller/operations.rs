use std::collections::HashSet;

use indexmap::IndexSet;

use crate::{grid::*, Array, CellValue, Pos};

use super::{
    formatting::CellFmtArray,
    operation::Operation,
    transaction_summary::{CellSheetsModified, TransactionSummary},
    update_code_cell_value::fetch_code_cell_difference,
    GridController,
};

impl GridController {
    /// Executes the given operation and returns the reverse operation.
    /// The only way to modify the internal state of the grid is through this function, with an operation.
    /// Operations must always return a reverse operation that can be used to undo the operation.
    pub fn execute_operation(
        &mut self,
        op: Operation,
        cells_to_compute: &mut IndexSet<CellRef>,
        summary: &mut TransactionSummary,
        sheets_with_changed_bounds: &mut HashSet<SheetId>,
        compute: bool,
    ) -> Operation {
        let operation = match op {
            Operation::None => Operation::None,
            Operation::SetCellValues { region, values } => {
                sheets_with_changed_bounds.insert(region.sheet);
                let sheet = self.grid.sheet_mut_from_id(region.sheet);

                let size = region.size().expect("msg: error getting size of region");
                let old_values = region
                    .iter()
                    .zip(values.into_cell_values_vec())
                    .map(|(cell_ref, value)| {
                        let pos = sheet.cell_ref_to_pos(cell_ref)?;
                        cells_to_compute.insert(cell_ref);
                        let response = sheet.set_cell_value(pos, value)?;
                        Some(response.old_value)
                    })
                    .map(|old_value| old_value.unwrap_or(CellValue::Blank))
                    .collect();

                let old_values = Array::new_row_major(size, old_values)
                    .expect("error constructing array of old values for SetCells operation");
                CellSheetsModified::add_region(&mut summary.cell_sheets_modified, sheet, &region);
                summary.generate_thumbnail =
                    summary.generate_thumbnail || self.thumbnail_dirty_region(&region);
                // return reverse operation
                Operation::SetCellValues {
                    region,
                    values: old_values,
                }
            }
            Operation::SetCellCode {
                cell_ref,
                code_cell_value,
            } => {
                sheets_with_changed_bounds.insert(cell_ref.sheet);

                let sheet = self.grid.sheet_mut_from_id(cell_ref.sheet);
                let old_code_cell_value = sheet.get_code_cell_from_ref(cell_ref).cloned();
                let pos = if let Some(pos) = sheet.cell_ref_to_pos(cell_ref) {
                    pos
                } else {
                    return Operation::None;
                };

                // for compute, we keep the original cell output to avoid flashing of output (since values will be overridden once computation is complete)
                if compute {
                    if let Some(code_cell_value) = code_cell_value {
                        let updated_code_cell_value =
                            if let Some(old_code_cell_value) = old_code_cell_value.as_ref() {
                                let mut updated_code_cell_value = code_cell_value.clone();
                                updated_code_cell_value.output = old_code_cell_value.output.clone();
                                updated_code_cell_value
                            } else {
                                code_cell_value
                            };
                        sheet.set_code_cell_value(pos, Some(updated_code_cell_value));
                    } else {
                        sheet.set_code_cell_value(pos, code_cell_value);
                    }
                    cells_to_compute.insert(cell_ref);
                } else {
                    // need to update summary (cells_to_compute will be ignored)
                    fetch_code_cell_difference(
                        sheet,
                        pos,
                        old_code_cell_value.clone(),
                        code_cell_value.clone(),
                        summary,
                        cells_to_compute,
                    );
                    sheet.set_code_cell_value(pos, code_cell_value);
                }
                summary
                    .cell_sheets_modified
                    .insert(CellSheetsModified::new(sheet.id, pos));
                summary.code_cells_modified.insert(cell_ref.sheet);
                Operation::SetCellCode {
                    cell_ref,
                    code_cell_value: old_code_cell_value,
                }
            }
            Operation::SetCellFormats { region, attr } => {
                sheets_with_changed_bounds.insert(region.sheet);

                if let CellFmtArray::FillColor(_) = attr {
                    summary.fill_sheets_modified.push(region.sheet);
                }

                // todo: this is too slow -- perhaps call this again when we have a better way of setting multiple formats within an array
                // or when we get rid of CellRefs (which I think is the reason this is slow)
                // summary.generate_thumbnail =
                //     summary.generate_thumbnail || self.thumbnail_dirty_region(region.clone());

                let old_attr = match attr {
                    CellFmtArray::Align(align) => {
                        CellFmtArray::Align(self.set_cell_formats_for_type::<CellAlign>(
                            &region,
                            align,
                            Some(&mut summary.cell_sheets_modified),
                        ))
                    }

                    CellFmtArray::Wrap(wrap) => {
                        CellFmtArray::Wrap(self.set_cell_formats_for_type::<CellWrap>(
                            &region,
                            wrap,
                            Some(&mut summary.cell_sheets_modified),
                        ))
                    }
                    CellFmtArray::NumericFormat(num_fmt) => CellFmtArray::NumericFormat(
                        self.set_cell_formats_for_type::<NumericFormat>(
                            &region,
                            num_fmt,
                            Some(&mut summary.cell_sheets_modified),
                        ),
                    ),
                    CellFmtArray::NumericDecimals(num_decimals) => CellFmtArray::NumericDecimals(
                        self.set_cell_formats_for_type::<NumericDecimals>(
                            &region,
                            num_decimals,
                            Some(&mut summary.cell_sheets_modified),
                        ),
                    ),
                    CellFmtArray::NumericCommas(num_commas) => CellFmtArray::NumericCommas(
                        self.set_cell_formats_for_type::<NumericCommas>(
                            &region,
                            num_commas,
                            Some(&mut summary.cell_sheets_modified),
                        ),
                    ),
                    CellFmtArray::Bold(bold) => {
                        CellFmtArray::Bold(self.set_cell_formats_for_type::<Bold>(
                            &region,
                            bold,
                            Some(&mut summary.cell_sheets_modified),
                        ))
                    }
                    CellFmtArray::Italic(italic) => {
                        CellFmtArray::Italic(self.set_cell_formats_for_type::<Italic>(
                            &region,
                            italic,
                            Some(&mut summary.cell_sheets_modified),
                        ))
                    }
                    CellFmtArray::TextColor(text_color) => {
                        CellFmtArray::TextColor(self.set_cell_formats_for_type::<TextColor>(
                            &region,
                            text_color,
                            Some(&mut summary.cell_sheets_modified),
                        ))
                    }
                    CellFmtArray::FillColor(fill_color) => {
                        summary.fill_sheets_modified.push(region.sheet);
                        CellFmtArray::FillColor(
                            self.set_cell_formats_for_type::<FillColor>(&region, fill_color, None),
                        )
                    }
                };

                // return reverse operation
                Operation::SetCellFormats {
                    region,
                    attr: old_attr,
                }
            }
            Operation::SetBorders { region, borders } => {
                sheets_with_changed_bounds.insert(region.sheet);
                summary.border_sheets_modified.push(region.sheet);
                summary.generate_thumbnail =
                    summary.generate_thumbnail || self.thumbnail_dirty_region(&region);

                let sheet = self.grid.sheet_mut_from_id(region.sheet);

                let old_borders = sheet.set_region_borders(&region, borders);
                Operation::SetBorders {
                    region,
                    borders: old_borders,
                }
            }
            Operation::AddSheet { sheet } => {
                // todo: need to handle the case where sheet.order overlaps another sheet order
                // this may happen after (1) delete a sheet; (2) MP update w/an added sheet; and (3) undo the deleted sheet
                let sheet_id = sheet.id;
                self.grid
                    .add_sheet(Some(sheet))
                    .expect("duplicate sheet name");
                summary.sheet_list_modified = true;

                // return reverse operation
                Operation::DeleteSheet { sheet_id }
            }
            Operation::DeleteSheet { sheet_id } => {
                let deleted_sheet = self.grid.remove_sheet(sheet_id);
                summary.sheet_list_modified = true;

                // return reverse operation
                Operation::AddSheet {
                    sheet: deleted_sheet,
                }
            }
            Operation::ReorderSheet { target, order } => {
                let old_first = self.grid.first_sheet_id();
                let sheet = self.grid.sheet_from_id(target);
                let original_order = sheet.order.clone();
                self.grid.move_sheet(target, order);
                summary.sheet_list_modified = true;

                if old_first != self.grid.first_sheet_id() {
                    summary.generate_thumbnail = true;
                }

                // return reverse operation
                Operation::ReorderSheet {
                    target,
                    order: original_order,
                }
            }
            Operation::SetSheetName { sheet_id, name } => {
                let sheet = self.grid.sheet_mut_from_id(sheet_id);
                let old_name = sheet.name.clone();
                sheet.name = name;
                summary.sheet_list_modified = true;

                // return reverse operation
                Operation::SetSheetName {
                    sheet_id,
                    name: old_name,
                }
            }
            Operation::SetSheetColor { sheet_id, color } => {
                let sheet = self.grid.sheet_mut_from_id(sheet_id);
                let old_color = sheet.color.clone();
                sheet.color = color;
                summary.sheet_list_modified = true;

                // return reverse operation
                Operation::SetSheetColor {
                    sheet_id,
                    color: old_color,
                }
            }

            Operation::ResizeColumn {
                sheet_id,
                column,
                new_size,
            } => {
                let sheet = self.grid.sheet_mut_from_id(sheet_id);
                if let Some(x) = sheet.get_column_index(column) {
                    summary.offsets_modified.push(sheet.id);
                    let old_size = sheet.offsets.set_column_width(x, new_size);
                    summary.generate_thumbnail = summary.generate_thumbnail
                        || self.thumbnail_dirty_pos(sheet_id, Pos { x, y: 0 });
                    Operation::ResizeColumn {
                        sheet_id,
                        column,
                        new_size: old_size,
                    }
                } else {
                    Operation::None
                }
            }

            Operation::ResizeRow {
                sheet_id,
                row,
                new_size,
            } => {
                let sheet = self.grid.sheet_mut_from_id(sheet_id);
                if let Some(y) = sheet.get_row_index(row) {
                    let old_size = sheet.offsets.set_row_height(y, new_size);
                    summary.offsets_modified.push(sheet.id);
                    summary.generate_thumbnail = summary.generate_thumbnail
                        || self.thumbnail_dirty_pos(sheet_id, Pos { x: 0, y });
                    Operation::ResizeRow {
                        sheet_id,
                        row,
                        new_size: old_size,
                    }
                } else {
                    Operation::None
                }
            }
        };
        operation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn execute(gc: &mut GridController, operation: Operation) {
        let mut cells_to_compute = IndexSet::new();
        let mut summary = TransactionSummary::default();
        let mut sheets_with_changed_bounds = HashSet::new();
        gc.execute_operation(
            operation,
            &mut cells_to_compute,
            &mut summary,
            &mut sheets_with_changed_bounds,
            false,
        );
    }

    #[test]
    fn test_execute_operation_set_sheet_color() {
        let mut gc = GridController::new();
        let sheet_id = gc.sheet_ids()[0];
        let color = Some("red".to_string());
        let operation = Operation::SetSheetColor {
            sheet_id,
            color: color.clone(),
        };

        assert_eq!(
            format!("{:?}", operation),
            format!(
                "SetSheetColor {{ sheet_id: SheetId {{ id: {} }}, color: Some(\"red\") }}",
                sheet_id
            )
        );

        execute(&mut gc, operation);
        assert_eq!(gc.grid.sheets()[0].color, color);
    }

    #[test]
    fn test_execute_operation_resize_column() {
        let mut gc = GridController::new();
        let sheet = &mut gc.grid_mut().sheets_mut()[0];
        let (_, column) = sheet.get_or_create_column(0);
        let column_id = column.id;
        let sheet_id = sheet.id;
        let new_size = 100.0;
        let operation = Operation::ResizeColumn {
            sheet_id,
            column: column_id,
            new_size,
        };

        assert_eq!(
            format!("{:?}", operation),
            format!(
                "ResizeColumn {{ sheet_id: SheetId {{ id: {} }}, column: ColumnId {{ id: {} }}, new_size: {:.1} }}",
                sheet_id, column_id, new_size
            )
        );

        execute(&mut gc, operation);
        let column_width = gc.grid.sheets()[0].offsets.column_width(0);
        assert_eq!(column_width, new_size);
    }

    #[test]
    fn test_execute_operation_resize_row() {
        let mut gc = GridController::new();
        let sheet = &mut gc.grid_mut().sheets_mut()[0];
        let row = sheet.get_or_create_row(0);
        let row_id = row.id;
        let sheet_id = sheet.id;
        let new_size = 100.0;
        let operation = Operation::ResizeRow {
            sheet_id,
            row: row_id,
            new_size,
        };

        assert_eq!(
            format!("{:?}", operation),
            format!(
                "ResizeRow {{ sheet_id: SheetId {{ id: {} }}, row: RowId {{ id: {} }}, new_size: {:.1} }}",
                sheet_id, row_id, new_size
            )
        );

        execute(&mut gc, operation);
        let row_height = gc.grid.sheets()[0].offsets.row_height(0);
        assert_eq!(row_height, new_size);
    }
}