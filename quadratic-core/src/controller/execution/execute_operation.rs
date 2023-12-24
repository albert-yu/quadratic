use crate::grid::formatting::CellFmtArray;
use crate::{grid::*, Array, CellValue, SheetPos};

use crate::controller::operations::operation::Operation;
use crate::controller::GridController;

impl GridController {
    /// Executes the given operation.
    /// Returns true if the operation resulted in an async call.
    ///
    pub fn execute_operation(&mut self, op: Operation) {
        match op.clone() {
            Operation::SetCellValues { sheet_rect, values } => {
                let sheet = self.grid.sheet_mut_from_id(sheet_rect.sheet_id);

                // update individual cell values and collect old_values
                let old_values = sheet_rect
                    .iter()
                    .zip(values.clone().into_cell_values_vec())
                    .map(|(sheet_pos, value)| {
                        let old_value = sheet.set_cell_value(sheet_pos.into(), value);

                        // add html to summary if old value was of that type
                        if old_value
                            .as_ref()
                            .is_some_and(|cell_value| cell_value.is_html())
                        {
                            self.summary.html.insert(sheet_pos.sheet_id);
                        }
                        old_value
                    })
                    .map(|old_value| old_value.unwrap_or(CellValue::Blank))
                    .collect();

                self.forward_operations.push(op);

                // create reverse_operation
                let old_values = Array::new_row_major(sheet_rect.size(), old_values)
                    .expect("error constructing array of old values for SetCells operation");
                self.reverse_operations.push(Operation::SetCellValues {
                    sheet_rect,
                    values: old_values,
                });

                self.add_compute_operations(&sheet_rect, None);

                self.check_spills(&sheet_rect);

                // prepare summary
                self.sheets_with_dirty_bounds.insert(sheet_rect.sheet_id);
                self.summary.generate_thumbnail =
                    self.summary.generate_thumbnail || self.thumbnail_dirty_sheet_rect(sheet_rect);
                self.add_cell_sheets_modified_rect(&sheet_rect);
            }

            Operation::SetCodeCell {
                sheet_pos,
                code_cell_value,
            } => {
                let sheet_id = sheet_pos.sheet_id;
                let sheet = self.grid.sheet_mut_from_id(sheet_id);
                sheet.set_code_cell(sheet_pos.into(), code_cell_value);

                let sheet_rect = sheet_pos.into();
                self.sheets_with_dirty_bounds.insert(sheet_id);
                self.summary.generate_thumbnail =
                    self.summary.generate_thumbnail || self.thumbnail_dirty_sheet_rect(sheet_rect);
                self.summary.code_cells_modified.insert(sheet_id);
                self.add_cell_sheets_modified_rect(&sheet_rect);
            }

            Operation::DeleteCodeCell { sheet_pos } => {
                let sheet_id = sheet_pos.sheet_id;
                let sheet = self.grid.sheet_mut_from_id(sheet_id);
                self.reverse_operations.push(Operation::SetCodeCell {
                    sheet_pos,
                    code_cell_value: sheet.get_code_cell(sheet_pos.into()).cloned(),
                });

                sheet.set_code_cell(sheet_pos.into(), None);
                self.forward_operations.push(Operation::SetCodeCell {
                    sheet_pos,
                    code_cell_value: None,
                });
                self.check_spills(&sheet_pos.into());

                let sheet_rect = sheet_pos.into();
                self.sheets_with_dirty_bounds.insert(sheet_id);
                self.summary.generate_thumbnail =
                    self.summary.generate_thumbnail || self.thumbnail_dirty_sheet_rect(sheet_rect);
                self.summary.code_cells_modified.insert(sheet_id);
                self.add_cell_sheets_modified_rect(&sheet_rect);
            }

            Operation::ComputeCodeCell {
                sheet_pos,
                code_cell_value,
            } => {
                // only execute if sheet still exists
                if let Some(sheet) = self.grid.try_sheet_mut_from_id(sheet_pos.sheet_id) {
                    let old_code_cell =
                        sheet.set_code_cell(sheet_pos.into(), Some(code_cell_value.clone()));

                    match code_cell_value.language {
                        CodeCellLanguage::Python => {
                            self.run_python(sheet_pos, &code_cell_value);
                        }
                        CodeCellLanguage::Formula => {
                            self.run_formula(sheet_pos, &code_cell_value);
                        }
                        _ => {
                            unreachable!("Unsupported language in RunCodeCell");
                        }
                    }

                    // only capture operations if not waiting for async, otherwise wait until calculation is complete
                    if self.waiting_for_async.is_none() {
                        self.finalize_code_cell(sheet_pos, old_code_cell);
                    }
                }
            }

            Operation::SetCellFormats { sheet_rect, attr } => {
                self.sheets_with_dirty_bounds.insert(sheet_rect.sheet_id);

                if let CellFmtArray::FillColor(_) = attr {
                    self.summary.fill_sheets_modified.push(sheet_rect.sheet_id);
                }

                // todo: this is too slow -- perhaps call this again when we have a better way of setting multiple formats within an array
                // or when we get rid of CellRefs (which I think is the reason this is slow)
                // summary.generate_thumbnail =
                //     summary.generate_thumbnail || self.thumbnail_dirty_region(region.clone());

                let old_attr = match attr.clone() {
                    CellFmtArray::Align(align) => CellFmtArray::Align(
                        self.set_cell_formats_for_type::<CellAlign>(&sheet_rect, align, true),
                    ),
                    CellFmtArray::Wrap(wrap) => CellFmtArray::Wrap(
                        self.set_cell_formats_for_type::<CellWrap>(&sheet_rect, wrap, true),
                    ),
                    CellFmtArray::NumericFormat(num_fmt) => CellFmtArray::NumericFormat(
                        self.set_cell_formats_for_type::<NumericFormat>(&sheet_rect, num_fmt, true),
                    ),
                    CellFmtArray::NumericDecimals(num_decimals) => CellFmtArray::NumericDecimals(
                        self.set_cell_formats_for_type::<NumericDecimals>(
                            &sheet_rect,
                            num_decimals,
                            true,
                        ),
                    ),
                    CellFmtArray::NumericCommas(num_commas) => CellFmtArray::NumericCommas(
                        self.set_cell_formats_for_type::<NumericCommas>(
                            &sheet_rect,
                            num_commas,
                            true,
                        ),
                    ),
                    CellFmtArray::Bold(bold) => CellFmtArray::Bold(
                        self.set_cell_formats_for_type::<Bold>(&sheet_rect, bold, true),
                    ),
                    CellFmtArray::Italic(italic) => CellFmtArray::Italic(
                        self.set_cell_formats_for_type::<Italic>(&sheet_rect, italic, true),
                    ),
                    CellFmtArray::TextColor(text_color) => CellFmtArray::TextColor(
                        self.set_cell_formats_for_type::<TextColor>(&sheet_rect, text_color, true),
                    ),
                    CellFmtArray::FillColor(fill_color) => {
                        self.summary.fill_sheets_modified.push(sheet_rect.sheet_id);
                        CellFmtArray::FillColor(self.set_cell_formats_for_type::<FillColor>(
                            &sheet_rect,
                            fill_color,
                            false,
                        ))
                    }
                    CellFmtArray::RenderSize(output_size) => {
                        self.summary.html.insert(sheet_rect.sheet_id);
                        CellFmtArray::RenderSize(self.set_cell_formats_for_type::<RenderSize>(
                            &sheet_rect,
                            output_size,
                            false,
                        ))
                    }
                };

                self.forward_operations
                    .push(Operation::SetCellFormats { sheet_rect, attr });

                self.reverse_operations.push(Operation::SetCellFormats {
                    sheet_rect,
                    attr: old_attr,
                });
            }

            Operation::SetBorders {
                sheet_rect,
                borders,
            } => {
                self.sheets_with_dirty_bounds.insert(sheet_rect.sheet_id);
                self.summary
                    .border_sheets_modified
                    .push(sheet_rect.sheet_id);
                self.summary.generate_thumbnail =
                    self.summary.generate_thumbnail || self.thumbnail_dirty_sheet_rect(sheet_rect);

                let sheet = self.grid.sheet_mut_from_id(sheet_rect.sheet_id);

                let old_borders = sheet.set_region_borders(&sheet_rect.into(), borders.clone());

                // should be removed
                self.forward_operations.push(Operation::SetBorders {
                    sheet_rect,
                    borders,
                });
                self.reverse_operations.push(Operation::SetBorders {
                    sheet_rect,
                    borders: old_borders,
                });
            }
            Operation::AddSheet { sheet } => {
                // todo: need to handle the case where sheet.order overlaps another sheet order
                // this may happen after (1) delete a sheet; (2) MP update w/an added sheet; and (3) undo the deleted sheet
                let sheet_id = sheet.id;
                self.grid
                    .add_sheet(Some(sheet.clone()))
                    .expect("duplicate sheet name");
                self.summary.sheet_list_modified = true;
                self.summary.html.insert(sheet_id);
                self.forward_operations.push(Operation::AddSheet { sheet });
                self.reverse_operations
                    .push(Operation::DeleteSheet { sheet_id });
            }
            Operation::DeleteSheet { sheet_id } => {
                let deleted_sheet = self.grid.remove_sheet(sheet_id);
                self.summary.sheet_list_modified = true;
                self.forward_operations
                    .push(Operation::DeleteSheet { sheet_id });
                self.reverse_operations.push(Operation::AddSheet {
                    sheet: deleted_sheet,
                });
            }
            Operation::ReorderSheet { target, order } => {
                let old_first = self.grid.first_sheet_id();
                let sheet = self.grid.sheet_from_id(target);
                let original_order = sheet.order.clone();
                self.grid.move_sheet(target, order.clone());
                self.summary.sheet_list_modified = true;

                if old_first != self.grid.first_sheet_id() {
                    self.summary.generate_thumbnail = true;
                }
                self.forward_operations
                    .push(Operation::ReorderSheet { target, order });
                self.reverse_operations.push(Operation::ReorderSheet {
                    target,
                    order: original_order,
                });
            }
            Operation::SetSheetName { sheet_id, name } => {
                let sheet = self.grid.sheet_mut_from_id(sheet_id);
                let old_name = sheet.name.clone();
                sheet.name = name.clone();
                self.summary.sheet_list_modified = true;
                self.forward_operations
                    .push(Operation::SetSheetName { sheet_id, name });
                self.reverse_operations.push(Operation::SetSheetName {
                    sheet_id,
                    name: old_name,
                });
            }
            Operation::SetSheetColor { sheet_id, color } => {
                let sheet = self.grid.sheet_mut_from_id(sheet_id);
                let old_color = sheet.color.clone();
                sheet.color = color.clone();
                self.summary.sheet_list_modified = true;
                self.forward_operations
                    .push(Operation::SetSheetColor { sheet_id, color });
                self.reverse_operations.push(Operation::SetSheetColor {
                    sheet_id,
                    color: old_color,
                });
            }

            Operation::ResizeColumn {
                sheet_id,
                column,
                new_size,
            } => {
                let sheet = self.grid.sheet_mut_from_id(sheet_id);
                self.summary.offsets_modified.push(sheet.id);
                let old_size = sheet.offsets.set_column_width(column, new_size);
                self.summary.generate_thumbnail = self.summary.generate_thumbnail
                    || self.thumbnail_dirty_sheet_pos(SheetPos {
                        x: column,
                        y: 0,
                        sheet_id,
                    });
                self.forward_operations.push(Operation::ResizeColumn {
                    sheet_id,
                    column,
                    new_size,
                });
                self.reverse_operations.push(Operation::ResizeColumn {
                    sheet_id,
                    column,
                    new_size: old_size,
                });
            }

            Operation::ResizeRow {
                sheet_id,
                row,
                new_size,
            } => {
                let sheet = self.grid.sheet_mut_from_id(sheet_id);
                let old_size = sheet.offsets.set_row_height(row, new_size);
                self.summary.offsets_modified.push(sheet.id);
                self.summary.generate_thumbnail = self.summary.generate_thumbnail
                    || self.thumbnail_dirty_sheet_pos(SheetPos {
                        x: 0,
                        y: row,
                        sheet_id,
                    });
                self.forward_operations.push(Operation::ResizeRow {
                    sheet_id,
                    row,
                    new_size,
                });
                self.reverse_operations.push(Operation::ResizeRow {
                    sheet_id,
                    row,
                    new_size: old_size,
                });
            }
        }
    }
}

/*

    /*


    #[test]
    fn test_spilled_output_over_code_cell() {
        let mut gc = GridController::new();
        let sheet_id = gc.sheet_ids()[0];

        let code_cell_output = Some(CodeCellValue {
            language: CodeCellLanguage::Python,
            code_string: "print(1)".to_string(),
            formatted_code_string: None,
            last_modified: String::from(""),
            output: Some(CodeCellRunOutput {
                std_out: None,
                std_err: None,
                result: crate::grid::CodeCellRunResult::Ok {
                    output_value: Value::Array(Array::new_empty(
                        ArraySize::try_from((2, 3)).expect("failed to create array"),
                    )),
                    cells_accessed: HashSet::new(),
                },
                spill: false,
            }),
        });

        let original = SheetPos {
            x: 0,
            y: 1,
            sheet_id,
        };
        gc.transaction_in_progress = true;
        gc.update_code_cell_value(original, code_cell_output.clone());

        let code_cell_output = Some(CodeCellValue {
            language: CodeCellLanguage::Python,
            code_string: "print(1)".to_string(),
            formatted_code_string: None,
            last_modified: String::from(""),
            output: Some(CodeCellRunOutput {
                std_out: None,
                std_err: None,
                result: crate::grid::CodeCellRunResult::Ok {
                    output_value: Value::Array(Array::new_empty(
                        ArraySize::try_from((1, 3)).expect("failed to create array"),
                    )),
                    cells_accessed: HashSet::new(),
                },
                spill: false,
            }),
        });

        let sheet_pos = SheetPos {
            x: 0,
            y: 0,
            sheet_id,
        };

        gc.update_code_cell_value(sheet_pos, code_cell_output.clone());

        let sheet = gc.grid.sheet_from_id(sheet_id);
        let code_cell = sheet.get_code_cell(Pos { x: 0, y: 0 });
        assert!(code_cell.unwrap().has_spill_error());
        assert_eq!(
            sheet.get_column(0).unwrap().spills.get(0),
            Some(sheet_pos.into())
        );
        assert_eq!(
            sheet.get_column(0).unwrap().spills.get(1),
            Some(original.into())
        );
    }
     */
}
 */
