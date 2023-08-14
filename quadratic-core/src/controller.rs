use itertools::Itertools;
use serde::{Deserialize, Serialize};
#[cfg(feature = "js")]
use wasm_bindgen::prelude::*;

use crate::{
    grid::{CellRef, ColumnId, Grid, RowId, Sheet, SheetId},
    Array, CellValue, Pos, Rect,
};

#[derive(Debug, Default, Clone)]
#[cfg_attr(feature = "js", wasm_bindgen)]
pub struct GridController {
    grid: Grid,
    undo_stack: Vec<Transaction>,
    redo_stack: Vec<Transaction>,
}
impl GridController {
    pub fn new() -> Self {
        Self::from_grid(Grid::new())
    }
    pub fn from_grid(grid: Grid) -> Self {
        GridController {
            grid,
            undo_stack: vec![],
            redo_stack: vec![],
        }
    }
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    pub fn sheet_ids(&self) -> Vec<SheetId> {
        self.grid.sheets().iter().map(|sheet| sheet.id).collect()
    }
    pub fn sheet(&self, sheet_id: SheetId) -> &Sheet {
        self.grid.sheet_from_id(sheet_id)
    }

    pub fn set_cell_value(
        &mut self,
        sheet_id: SheetId,
        pos: Pos,
        value: CellValue,
    ) -> TransactionSummary {
        let sheet = self.grid.sheet_mut_from_id(sheet_id);
        let cell_ref = sheet.get_or_create_cell_ref(pos);
        let transaction = Transaction {
            ops: vec![Operation::SetCell { cell_ref, value }],
        };
        self.transact_forward(transaction)
    }
    pub fn set_cells(
        &mut self,
        sheet_id: SheetId,
        start_pos: Pos,
        values: Array,
    ) -> TransactionSummary {
        let sheet = self.grid.sheet_mut_from_id(sheet_id);
        let region = Rect {
            min: start_pos,
            max: Pos {
                x: start_pos.x + values.width() as i64 - 1,
                y: start_pos.y + values.height() as i64 - 1,
            },
        };
        let columns = region
            .x_range()
            .map(|x| sheet.get_or_create_column(x).0.id)
            .collect();
        let rows = region
            .x_range()
            .map(|x| sheet.get_or_create_row(x).id)
            .collect();
        let transaction = Transaction {
            ops: vec![Operation::SetCells {
                sheet_id,
                columns,
                rows,
                values,
            }],
        };
        self.transact_forward(transaction)
    }
    pub fn delete_cell_values(&mut self, sheet_id: SheetId, region: Rect) -> TransactionSummary {
        let sheet = self.grid.sheet_from_id(sheet_id);
        let columns = region
            .x_range()
            .filter_map(|x| sheet.get_column(x))
            .map(|column| column.id)
            .collect_vec();
        let rows = region
            .x_range()
            .filter_map(|x| sheet.get_row(x))
            .collect_vec();
        let width = columns.len() as u32;
        let height = rows.len() as u32;
        let transaction = match Array::new_empty(width, height) {
            Ok(values) => Transaction {
                ops: vec![Operation::SetCells {
                    sheet_id,
                    columns,
                    rows,
                    values,
                }],
            },
            Err(_) => Transaction {
                ops: vec![], // nothing to do! TODO: probably shouldn't even have an undo stack entry
            },
        };
        self.transact_forward(transaction)
    }

    pub fn add_sheet(&mut self) -> TransactionSummary {
        let sheet_names = &self
            .grid
            .sheets()
            .iter()
            .map(|s| s.name.as_str())
            .collect_vec();

        let id = SheetId::new();
        let name = crate::util::unused_name("Sheet", &sheet_names);

        let transaction = Transaction::from(Operation::AddSheet {
            sheet: Sheet::new(id, name),
            before: None,
        });
        self.transact_forward(transaction)
    }
    pub fn delete_sheet(&mut self, sheet_id: SheetId) -> TransactionSummary {
        let transaction = Transaction::from(Operation::DeleteSheet { sheet_id });
        self.transact_forward(transaction)
    }
    pub fn move_sheet(
        &mut self,
        sheet_id: SheetId,
        to_before: Option<SheetId>,
    ) -> TransactionSummary {
        let transaction = Transaction::from(Operation::ReorderSheet {
            target: sheet_id,
            to_before,
        });
        self.transact_forward(transaction)
    }

    fn transact_forward(&mut self, transaction: Transaction) -> TransactionSummary {
        let (reverse_transaction, summary) = self.transact(transaction);
        self.redo_stack.clear();
        self.undo_stack.push(reverse_transaction);
        summary
    }
    pub fn has_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }
    pub fn has_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
    pub fn undo(&mut self) -> Option<TransactionSummary> {
        let transaction = self.undo_stack.pop()?;
        let (reverse_transaction, summary) = self.transact(transaction);
        self.redo_stack.push(reverse_transaction);
        Some(summary)
    }
    pub fn redo(&mut self) -> Option<TransactionSummary> {
        let transaction = self.redo_stack.pop()?;
        let (reverse_transaction, summary) = self.transact(transaction);
        self.undo_stack.push(reverse_transaction);
        Some(summary)
    }

    fn transact(&mut self, transaction: Transaction) -> (Transaction, TransactionSummary) {
        let mut rev_ops = vec![];
        let mut sheets_with_changed_bounds = vec![];
        let mut summary = TransactionSummary::default();
        for op in transaction.ops {
            if let Some(new_dirty_sheet) = op.sheet_with_changed_bounds() {
                if !sheets_with_changed_bounds.contains(&new_dirty_sheet) {
                    sheets_with_changed_bounds.push(new_dirty_sheet)
                }
            }
            match op {
                Operation::SetCell { cell_ref, value } => {
                    let sheet = self.grid.sheet_mut_from_id(cell_ref.sheet);
                    let cell_xy = sheet.cell_ref_to_pos(cell_ref).expect("bad cell reference");

                    summary
                        .cell_regions_modified
                        .push((cell_ref.sheet, Rect::single_pos(cell_xy)));

                    let old_value = match sheet.set_cell_value(cell_xy, value) {
                        Some(response) => response.old_value,
                        None => CellValue::Blank,
                    };
                    rev_ops.push(Operation::SetCell {
                        cell_ref,
                        value: old_value,
                    });
                }

                Operation::SetCells {
                    sheet_id,
                    columns,
                    rows,
                    values,
                } => {
                    let sheet = self.grid.sheet_mut_from_id(sheet_id);

                    // Perhaps some columns or rows have disappeared since this
                    // transaction was first made. In that case, ignore the
                    // deleted columns when considering what region was
                    // modified.
                    let xs = columns.iter().filter_map(|&id| sheet.get_column_index(id));
                    let ys = rows.iter().filter_map(|&id| sheet.get_row_index(id));
                    if let Some(modified_rect) = Rect::from_xs_and_ys(xs, ys) {
                        summary
                            .cell_regions_modified
                            .push((sheet_id, modified_rect));
                    }

                    let width = rows.len() as u32;
                    let height = columns.len() as u32;
                    let old_values = itertools::iproduct!(&rows, &columns)
                        .zip(values.into_cell_values_vec())
                        .map(|((&row, &column), value)| {
                            let pos = sheet.cell_ref_to_pos(CellRef {
                                sheet: sheet_id,
                                column,
                                row,
                            })?;
                            let response = sheet.set_cell_value(pos, value)?;
                            Some(response.old_value)
                        })
                        .map(|old_value| old_value.unwrap_or(CellValue::Blank))
                        .collect();
                    let old_values = Array::new_row_major(width, height, old_values)
                        .expect("error constructing array of old values for SetCells operation");
                    rev_ops.push(Operation::SetCells {
                        sheet_id,
                        columns,
                        rows,
                        values: old_values,
                    })
                }

                Operation::AddSheet { sheet, before } => {
                    let sheet_id = sheet.id;
                    self.grid
                        .add_sheet(sheet, before.and_then(|id| self.grid.sheet_id_to_index(id)))
                        .expect("duplicate sheet name");
                    rev_ops.push(Operation::DeleteSheet { sheet_id });
                }
                Operation::DeleteSheet { sheet_id } => {
                    let old_after = self
                        .grid
                        .sheet_id_to_index(sheet_id)
                        .and_then(|i| Some(*self.sheet_ids().get(i + 1)?));
                    let deleted_sheet = self.grid.remove_sheet(sheet_id);
                    if let Some(sheet) = deleted_sheet {
                        rev_ops.push(Operation::AddSheet {
                            sheet,
                            before: old_after,
                        });
                    }
                }

                Operation::ReorderSheet { target, to_before } => {
                    // TODO: This should probably use fractional indexing to be
                    // more robust. Fortunately the order of sheets is not too
                    // high-stakes.
                    //
                    // Right now, if `to_before` doesn't exist, the operation
                    // just does nothing.
                    let old_position = self.sheet_ids().iter().position(|&id| id == target);
                    let new_position = match to_before {
                        Some(to_before) => self.sheet_ids().iter().position(|&id| id == to_before),
                        None => Some(self.sheet_ids().len()),
                    };
                    if let (Some(old), Some(new)) = (old_position, new_position) {
                        summary.sheet_list_modified = true;
                        let old_after = self.sheet_ids().get(old + 1).copied();
                        self.grid.move_sheet(target, new);
                        rev_ops.push(Operation::ReorderSheet {
                            target,
                            to_before: old_after,
                        });
                    }
                }
            }
        }
        for dirty_sheet in sheets_with_changed_bounds {
            self.grid
                .sheet_mut_from_id(dirty_sheet)
                .recalculate_bounds();
        }
        rev_ops.reverse();

        let reverse_transaction = Transaction { ops: rev_ops };

        (reverse_transaction, summary)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    ops: Vec<Operation>,
}
impl From<Operation> for Transaction {
    fn from(op: Operation) -> Self {
        Transaction { ops: vec![op] }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Operation {
    SetCell {
        cell_ref: CellRef,
        value: CellValue,
    },
    SetCells {
        sheet_id: SheetId,
        columns: Vec<ColumnId>,
        rows: Vec<RowId>,
        values: Array,
    },

    AddSheet {
        sheet: Sheet,
        before: Option<SheetId>,
    },
    DeleteSheet {
        sheet_id: SheetId,
    },

    ReorderSheet {
        target: SheetId,
        to_before: Option<SheetId>,
    },
}
impl Operation {
    pub fn sheet_with_changed_bounds(&self) -> Option<SheetId> {
        match self {
            Operation::SetCell { cell_ref, .. } => Some(cell_ref.sheet),
            Operation::SetCells { sheet_id, .. } => Some(*sheet_id),

            Operation::AddSheet { .. } => None,
            Operation::DeleteSheet { .. } => None,

            Operation::ReorderSheet { .. } => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "js", derive(ts_rs::TS))]
pub struct TransactionSummary {
    /// Cell and text formatting regions modified.
    pub cell_regions_modified: Vec<(SheetId, Rect)>,
    /// Sheets where any fills have been modified.
    pub fill_sheets_modified: Vec<SheetId>,
    /// Sheets where any borders have been modified.
    pub border_sheets_modified: Vec<SheetId>,

    /// Locations of code cells that were modified. They may no longer exist.
    pub code_cells_modified: Vec<(SheetId, Pos)>,

    /// Sheet metadata or order was modified.
    pub sheet_list_modified: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_cell_undo_redo() {
        let mut g = GridController::new();
        let sheet_id = g.grid.sheets()[0].id;
        let pos = Pos { x: 3, y: 6 };
        let get_the_cell =
            |g: &GridController| g.sheet(sheet_id).get_cell_value(pos).unwrap_or_default();
        let expected_summary = Some(TransactionSummary {
            cell_regions_modified: vec![(sheet_id, Rect::single_pos(pos))],
            ..Default::default()
        });

        assert_eq!(get_the_cell(&g), CellValue::Blank);
        g.set_cell_value(sheet_id, pos, "a".into());
        assert_eq!(get_the_cell(&g), "a".into());
        g.set_cell_value(sheet_id, pos, "b".into());
        assert_eq!(get_the_cell(&g), "b".into());
        assert!(g.undo() == expected_summary);
        assert_eq!(get_the_cell(&g), "a".into());
        assert!(g.redo() == expected_summary);
        assert_eq!(get_the_cell(&g), "b".into());
        assert!(g.undo() == expected_summary);
        assert_eq!(get_the_cell(&g), "a".into());
        assert!(g.undo() == expected_summary);
        assert_eq!(get_the_cell(&g), CellValue::Blank);
        assert!(g.undo().is_none());
        assert_eq!(get_the_cell(&g), CellValue::Blank);
        assert!(g.redo() == expected_summary);
        assert_eq!(get_the_cell(&g), "a".into());
        assert!(g.redo() == expected_summary);
        assert_eq!(get_the_cell(&g), "b".into());
        assert!(g.redo().is_none());
        assert_eq!(get_the_cell(&g), "b".into());
    }

    #[test]
    fn test_add_delete_reorder_sheets() {
        let mut g = GridController::new();
        g.add_sheet();
        g.add_sheet();
        let old_sheet_ids = g.sheet_ids();
        let s1 = old_sheet_ids[0];
        let s2 = old_sheet_ids[1];
        let s3 = old_sheet_ids[2];

        let mut test_reorder = |a, b, expected: [SheetId; 3]| {
            g.move_sheet(a, b);
            assert_eq!(expected.to_vec(), g.sheet_ids());
            g.undo();
            assert_eq!(old_sheet_ids, g.sheet_ids());
        };

        test_reorder(s1, Some(s2), [s1, s2, s3]);
        test_reorder(s1, Some(s3), [s2, s1, s3]);
        test_reorder(s1, None, [s2, s3, s1]);
        test_reorder(s2, Some(s1), [s2, s1, s3]);
        test_reorder(s2, Some(s3), [s1, s2, s3]);
        test_reorder(s2, None, [s1, s3, s2]);
        test_reorder(s3, Some(s1), [s3, s1, s2]);
        test_reorder(s3, Some(s2), [s1, s3, s2]);
        test_reorder(s3, None, [s1, s2, s3]);

        let mut test_delete = |a, expected: [SheetId; 2]| {
            g.delete_sheet(a);
            assert_eq!(expected.to_vec(), g.sheet_ids());
            g.undo();
            assert_eq!(old_sheet_ids, g.sheet_ids());
        };

        test_delete(s1, [s2, s3]);
        test_delete(s2, [s1, s3]);
        test_delete(s3, [s1, s2]);
    }
}
