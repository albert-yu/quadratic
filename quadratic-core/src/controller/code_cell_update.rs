use crate::{
    grid::{CellRef, CodeCellValue, Sheet},
    ArraySize, Pos, Value,
};

use super::{
    operation::Operation,
    transaction_summary::{CellSheetsModified, TransactionSummary},
    GridController,
};

/// updates code cell value
/// returns true if the code cell was successful
pub fn update_code_cell_value(
    grid_controller: &mut GridController,
    cell_ref: CellRef,
    updated_code_cell_value: Option<CodeCellValue>,
    cells_to_compute: &mut Option<&mut Vec<CellRef>>,
    reverse_operations: &mut Vec<Operation>,
    summary: &mut TransactionSummary,
) -> bool {
    let mut success = false;
    let sheet = grid_controller.grid.sheet_mut_from_id(cell_ref.sheet);
    if let Some(pos) = sheet.cell_ref_to_pos(cell_ref) {
        let old_code_cell_value = sheet.set_code_cell_value(pos, updated_code_cell_value.clone());
        if let Some(updated_code_cell_value) = updated_code_cell_value.clone() {
            if let Some(output) = updated_code_cell_value.output {
                match output.result.output_value() {
                    Some(output_value) => {
                        success = true;
                        match output_value {
                            Value::Array(array) => {
                                for x in 0..array.size().w.into() {
                                    let column_id =
                                        sheet.get_or_create_column(pos.x + x as i64).0.id;
                                    for y in 0..array.size().h.into() {
                                        summary
                                            .cell_sheets_modified
                                            .insert(CellSheetsModified::new(sheet.id, pos.into()));
                                        let row_id = sheet.get_or_create_row(pos.y + y as i64).id;
                                        // add all but the first cell to the compute cycle
                                        if x != 0 && y != 0 {
                                            if let Some(cells_to_compute) = cells_to_compute {
                                                cells_to_compute.push(CellRef {
                                                    sheet: sheet.id,
                                                    column: column_id,
                                                    row: row_id,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                            Value::Single(..) => (),
                        };
                    }
                    None => {
                        summary
                            .cell_sheets_modified
                            .insert(CellSheetsModified::new(sheet.id, pos.into()));
                    }
                };
            }
        }
        fetch_code_cell_difference(
            sheet,
            pos,
            old_code_cell_value.clone(),
            updated_code_cell_value.clone(),
            summary,
            cells_to_compute,
        );
        reverse_operations.push(Operation::SetCellCode {
            cell_ref,
            code_cell_value: old_code_cell_value,
        });
        summary.code_cells_modified.insert(sheet.id);
    }
    success
}

/// Fetches the difference between the old and new code cell values and updates the UI
pub fn fetch_code_cell_difference(
    sheet: &mut Sheet,
    pos: Pos,
    old_code_cell_value: Option<CodeCellValue>,
    new_code_cell_value: Option<CodeCellValue>,
    summary: &mut TransactionSummary,
    cells_to_compute: &mut Option<&mut Vec<CellRef>>,
) {
    let old_size = if let Some(old_code_cell_value) = old_code_cell_value {
        old_code_cell_value.output_size()
    } else {
        ArraySize::_1X1
    };
    let new_size = if let Some(new_code_cell_value) = new_code_cell_value {
        new_code_cell_value.output_size()
    } else {
        ArraySize::_1X1
    };

    if old_size.w > new_size.w {
        for x in new_size.w.get()..old_size.w.get() {
            let column_id = sheet.get_or_create_column(pos.x + x as i64).0.id;
            for y in 0..new_size.h.get() {
                let row_id = sheet.get_or_create_row(pos.y + y as i64).id;
                let pos = Pos {
                    x: pos.x + x as i64,
                    y: pos.y + y as i64,
                };
                summary
                    .cell_sheets_modified
                    .insert(CellSheetsModified::new(sheet.id, pos.into()));
                if let Some(cells_to_compute) = cells_to_compute {
                    cells_to_compute.push(CellRef {
                        sheet: sheet.id,
                        column: column_id,
                        row: row_id,
                    });
                }
            }
        }
    }
    if old_size.h > new_size.h {
        for x in 0..old_size.w.get() {
            let column_id = sheet.get_or_create_column(pos.x + x as i64).0.id;
            for y in new_size.h.get()..old_size.h.get() {
                let row_id = sheet.get_or_create_row(pos.y + y as i64).id;
                let pos = Pos {
                    x: pos.x + x as i64,
                    y: pos.y + y as i64,
                };
                summary
                    .cell_sheets_modified
                    .insert(CellSheetsModified::new(sheet.id, pos.into()));
                if let Some(cells_to_compute) = cells_to_compute {
                    cells_to_compute.push(CellRef {
                        sheet: sheet.id,
                        column: column_id,
                        row: row_id,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        controller::{
            code_cell_update::fetch_code_cell_difference, transaction_summary::TransactionSummary,
        },
        grid::{CodeCellLanguage, CodeCellRunOutput, CodeCellValue, Sheet},
        Array, ArraySize, SheetPos, Value,
    };

    #[test]
    fn test_fetch_code_cell_difference() {
        let mut sheet = Sheet::test();

        let old = Some(CodeCellValue {
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
                    cells_accessed: Vec::new(),
                },
            }),
        });
        let new_smaller = Some(CodeCellValue {
            language: CodeCellLanguage::Python,
            code_string: "print(1)".to_string(),
            formatted_code_string: None,
            last_modified: String::from(""),
            output: Some(CodeCellRunOutput {
                std_out: None,
                std_err: None,
                result: crate::grid::CodeCellRunResult::Ok {
                    output_value: Value::Array(Array::new_empty(
                        ArraySize::try_from((1, 2)).expect("failed to create array"),
                    )),
                    cells_accessed: Vec::new(),
                },
            }),
        });

        let sheet_pos = SheetPos {
            x: 0,
            y: 0,
            sheet_id: sheet.id,
        };

        let mut summary = TransactionSummary::default();

        fetch_code_cell_difference(
            &mut sheet,
            sheet_pos.into(),
            old.clone(),
            new_smaller,
            &mut summary,
            &mut None,
        );
        assert_eq!(summary.cell_sheets_modified.len(), 1);

        summary.clear();

        let new_larger = Some(CodeCellValue {
            language: CodeCellLanguage::Python,
            code_string: "print(1)".to_string(),
            formatted_code_string: None,
            last_modified: String::from(""),
            output: Some(CodeCellRunOutput {
                std_out: None,
                std_err: None,
                result: crate::grid::CodeCellRunResult::Ok {
                    output_value: Value::Array(Array::new_empty(
                        ArraySize::try_from((5, 6)).expect("failed to create array"),
                    )),
                    cells_accessed: Vec::new(),
                },
            }),
        });

        super::fetch_code_cell_difference(
            &mut sheet,
            sheet_pos.into(),
            old,
            new_larger,
            &mut summary,
            &mut None,
        );
        assert_eq!(summary.cell_sheets_modified.len(), 0);
    }
}
