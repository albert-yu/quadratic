use uuid::Uuid;

use crate::controller::{
    operations::operation::Operation, transaction_summary::TransactionSummary, GridController,
};

use super::TransactionType;

impl GridController {
    pub fn received_transaction(
        &mut self,
        transaction_id: Uuid,
        sequence_num: u64,
        operations: Vec<Operation>,
    ) -> TransactionSummary {
        self.client_apply_transaction(transaction_id, sequence_num, operations)
    }

    fn rollback_unsaved_transactions(&mut self) {
        self.clear_summary();
        let operations = self
            .unsaved_transactions
            .iter()
            .rev()
            .map(|(_, undo)| undo.operations.clone())
            .collect::<Vec<_>>();
        operations
            .iter()
            .for_each(|o| self.start_transaction(o.to_vec(), None, TransactionType::Rollback));
    }

    fn reapply_unsaved_transactions(&mut self) {
        let operations = self
            .unsaved_transactions
            .iter()
            .rev()
            .map(|(forward, _)| forward.operations.clone())
            .collect::<Vec<_>>();
        operations
            .iter()
            .for_each(|o| self.start_transaction(o.to_vec(), None, TransactionType::Rollback));
    }

    /// Used by the server to apply transactions.
    /// The server owns the sequence_num, so there's no need to check or alter the execution order.
    pub fn server_apply_transaction(&mut self, operations: Vec<Operation>) {
        self.start_transaction(operations, None, TransactionType::Multiplayer);
    }

    /// Used by the client to ensure transactions are applied in order
    pub fn client_apply_transaction(
        &mut self,
        transaction_id: Uuid,

        // todo: check this and request transactions again if out of order
        sequence_num: u64,

        operations: Vec<Operation>,
    ) -> TransactionSummary {
        // first check if the received transaction is one of ours
        let existing = self
            .unsaved_transactions
            .iter_mut()
            .enumerate()
            .find(|(_, unsaved_transaction)| unsaved_transaction.0.id == transaction_id);
        if let Some((index, _)) = existing {
            // if transaction is the top of the unsaved_transactions, then only need to set the sequence_num
            if index as u64 == sequence_num - self.last_sequence_num - 1 {
                self.unsaved_transactions.remove(index);
                self.last_sequence_num = sequence_num;

                // nothing to render as we've already rendered this transaction
                return TransactionSummary::default();
            } else {
                crate::util::dbgjs(format!(
                    "Rust panic: out of order. Received {}, expected {}",
                    sequence_num - self.last_sequence_num - 1,
                    index
                ));
                todo!(
                    "Handle received transactions that are out of order with current transaction."
                );
            }
        }
        if !self.unsaved_transactions.is_empty() {
            self.rollback_unsaved_transactions();
            self.start_transaction(operations, None, TransactionType::Rollback);
            self.reapply_unsaved_transactions();
        } else {
            self.start_transaction(operations, None, TransactionType::Multiplayer);
        }
        self.last_sequence_num = sequence_num;

        self.transaction_updated_bounds();
        let mut summary = self.prepare_transaction_summary();
        summary.generate_thumbnail = false;
        summary.operations = None;
        summary.save = false;
        summary
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use uuid::Uuid;

    use crate::{
        controller::{operations::operation::Operation, GridController},
        CellValue, Pos, SheetPos,
    };

    #[test]
    fn test_multiplayer_hello_world() {
        let mut gc1 = GridController::new();
        let sheet_id = gc1.sheet_ids()[0];
        let summary = gc1.set_cell_value(
            SheetPos {
                x: 0,
                y: 0,
                sheet_id,
            },
            "Hello World".to_string(),
            None,
        );
        let sheet = gc1.grid().try_sheet_from_id(sheet_id).unwrap();
        assert_eq!(
            sheet.get_cell_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Text("Hello World".to_string()))
        );

        let transaction_id = Uuid::from_str(&summary.transaction_id.unwrap()).unwrap();
        let operations: Vec<Operation> =
            serde_json::from_str(&summary.operations.unwrap()).unwrap();

        // received our own transaction back
        gc1.received_transaction(transaction_id, 1, operations.clone());

        let sheet = gc1.grid().try_sheet_from_id(sheet_id).unwrap();
        assert_eq!(
            sheet.get_cell_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Text("Hello World".to_string()))
        );
        assert_eq!(gc1.unsaved_transactions.len(), 0);

        let mut gc2 = GridController::new();
        gc2.grid_mut().sheets_mut()[0].id = sheet_id;
        gc2.received_transaction(transaction_id, 1, operations);
        let sheet = gc2.grid().try_sheet_from_id(sheet_id).unwrap();
        assert_eq!(
            sheet.get_cell_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Text("Hello World".to_string()))
        );
    }

    #[test]
    fn test_apply_multiplayer_before_unsaved_transaction() {
        let mut gc1 = GridController::new();
        let sheet_id = gc1.sheet_ids()[0];
        gc1.set_cell_value(
            SheetPos {
                x: 0,
                y: 0,
                sheet_id,
            },
            "Hello World from 1".to_string(),
            None,
        );
        let sheet = gc1.grid().try_sheet_from_id(sheet_id).unwrap();
        assert_eq!(
            sheet.get_cell_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Text("Hello World from 1".to_string()))
        );

        let mut gc2 = GridController::new();
        // set gc2's sheet 1's id to gc1 sheet 1's id
        gc2.grid
            .try_sheet_mut_from_id(gc2.sheet_ids()[0])
            .unwrap()
            .id = sheet_id;
        let summary = gc2.set_cell_value(
            SheetPos {
                x: 0,
                y: 0,
                sheet_id,
            },
            "Hello World from 2".to_string(),
            None,
        );
        let sheet = gc2.grid().try_sheet_from_id(sheet_id).unwrap();
        assert_eq!(
            sheet.get_cell_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Text("Hello World from 2".to_string()))
        );

        let transaction_id = Uuid::from_str(&summary.transaction_id.unwrap()).unwrap();
        let operations = serde_json::from_str(&summary.operations.unwrap()).unwrap();

        // gc1 should apply gc2's cell value to 0,0 before its unsaved transaction
        // and then reapply its unsaved transaction, overwriting 0,0
        gc1.received_transaction(transaction_id, 1, operations);
        let sheet = gc1.grid.try_sheet_from_id(sheet_id).unwrap();
        assert_eq!(
            sheet.get_cell_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Text("Hello World from 1".to_string()))
        );
    }

    #[test]
    fn test_server_apply_transaction() {
        let mut client = GridController::new();
        let sheet_id = client.sheet_ids()[0];
        let summary = client.set_cell_value(
            SheetPos {
                x: 0,
                y: 0,
                sheet_id,
            },
            "Hello World".to_string(),
            None,
        );
        let sheet = client.grid().try_sheet_from_id(sheet_id).unwrap();
        assert_eq!(
            sheet.get_cell_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Text("Hello World".to_string()))
        );
        let operations: Vec<Operation> =
            serde_json::from_str(&summary.operations.unwrap()).unwrap();

        let mut server = GridController::new();
        server
            .grid
            .try_sheet_mut_from_id(server.sheet_ids()[0])
            .unwrap()
            .id = sheet_id;
        server.server_apply_transaction(operations);
        let sheet = server.grid.try_sheet_from_id(sheet_id).unwrap();
        assert_eq!(
            sheet.get_cell_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Text("Hello World".to_string()))
        );
    }
}
