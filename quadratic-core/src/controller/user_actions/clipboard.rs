use crate::controller::active_transactions::transaction_name::TransactionName;
use crate::controller::operations::clipboard::PasteSpecial;
use crate::controller::GridController;
use crate::selection::Selection;
use crate::{SheetPos, SheetRect};

// To view you clipboard contents, go to https://evercoder.github.io/clipboard-inspector/
// To decode the html, use https://codebeautify.org/html-decode-string

impl GridController {
    pub fn cut_to_clipboard(
        &mut self,
        selection: &Selection,
        cursor: Option<String>,
    ) -> Result<(String, String), String> {
        let (ops, plain_text, html) = self.cut_to_clipboard_operations(selection)?;
        self.start_user_transaction(ops, cursor, TransactionName::CutClipboard);
        Ok((plain_text, html))
    }

    pub fn paste_from_clipboard(
        &mut self,
        selection: Selection,
        plain_text: Option<String>,
        html: Option<String>,
        special: PasteSpecial,
        cursor: Option<String>,
    ) {
        // first try html
        if let Some(html) = html {
            if let Ok(ops) = self.paste_html_operations(&selection, html, special) {
                return self.start_user_transaction(ops, cursor, TransactionName::PasteClipboard);
            }
        }
        // if not quadratic html, then use the plain text
        // first try html
        if let Some(plain_text) = plain_text {
            let dest_pos = selection.origin();
            let ops = self.paste_plain_text_operations(dest_pos, plain_text, special);
            self.start_user_transaction(ops, cursor, TransactionName::PasteClipboard);
        }
    }

    pub fn move_cells(&mut self, source: SheetRect, dest: SheetPos, cursor: Option<String>) {
        let ops = self.move_cells_operations(source, dest);
        self.start_user_transaction(ops, cursor, TransactionName::PasteClipboard);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        color::Rgba,
        controller::GridController,
        grid::{
            generate_borders, js_types::CellFormatSummary, set_rect_borders, BorderSelection,
            BorderStyle, CellBorderLine, CodeCellLanguage, Sheet, SheetId,
        },
        CellValue, CodeCellValue, Pos, Rect, SheetPos, SheetRect,
    };
    use bigdecimal::BigDecimal;

    fn set_borders(sheet: &mut Sheet) {
        let selection = vec![BorderSelection::All];
        let style = BorderStyle {
            color: Rgba::color_from_str("#000000").unwrap(),
            line: CellBorderLine::Line1,
        };
        let rect = Rect::new_span(Pos { x: 0, y: 0 }, Pos { x: 0, y: 0 });
        let borders = generate_borders(sheet, &rect, selection, Some(style));
        set_rect_borders(sheet, &rect, borders);
    }

    fn set_cell_value(gc: &mut GridController, sheet_id: SheetId, value: &str, x: i64, y: i64) {
        gc.set_cell_value(SheetPos { x, y, sheet_id }, value.into(), None);
    }

    fn set_code_cell(
        gc: &mut GridController,
        sheet_id: SheetId,
        language: CodeCellLanguage,
        code: &str,
        x: i64,
        y: i64,
    ) {
        gc.set_code_cell(SheetPos { x, y, sheet_id }, language, code.into(), None);
    }

    fn set_formula_code_cell(
        gc: &mut GridController,
        sheet_id: SheetId,
        code: &str,
        x: i64,
        y: i64,
    ) {
        set_code_cell(gc, sheet_id, CodeCellLanguage::Formula, code, x, y);
    }

    #[test]
    fn test_copy_to_clipboard() {
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];

        set_cell_value(&mut gc, sheet_id, "1, 1", 1, 1);
        gc.set_cell_bold(
            SheetRect {
                min: Pos { x: 1, y: 1 },
                max: Pos { x: 1, y: 1 },
                sheet_id,
            },
            Some(true),
            None,
        );
        set_cell_value(&mut gc, sheet_id, "12", 3, 2);
        gc.set_cell_italic(
            SheetRect {
                min: Pos { x: 3, y: 2 },
                max: Pos { x: 3, y: 2 },
                sheet_id,
            },
            Some(true),
            None,
        );

        let rect = Rect {
            min: Pos { x: 1, y: 1 },
            max: Pos { x: 3, y: 2 },
        };

        let selection = Selection::rect(rect, sheet_id);
        let sheet = gc.sheet(sheet_id);
        let (plain_text, _) = sheet.copy_to_clipboard(&selection).unwrap();
        assert_eq!(plain_text, String::from("1, 1\t\t\n\t\t12"));

        let selection = Selection::rect(
            Rect::new_span(Pos { x: 0, y: 0 }, Pos { x: 3, y: 3 }),
            sheet_id,
        );
        let clipboard = sheet.copy_to_clipboard(&selection).unwrap();

        // paste using plain_text
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];
        gc.paste_from_clipboard(
            Selection::pos(0, 0, sheet_id),
            Some(clipboard.clone().0),
            None,
            PasteSpecial::None,
            None,
        );

        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.display_value(Pos { x: 1, y: 1 }),
            Some(CellValue::Text(String::from("1, 1")))
        );
        assert_eq!(
            sheet.display_value(Pos { x: 3, y: 2 }),
            Some(CellValue::Number(BigDecimal::from(12)))
        );

        // paste using html
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];
        gc.paste_from_clipboard(
            Selection::pos(0, 0, sheet_id),
            Some(String::from("")),
            Some(clipboard.clone().1),
            PasteSpecial::None,
            None,
        );
        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.display_value(Pos { x: 1, y: 1 }),
            Some(CellValue::Text(String::from("1, 1")))
        );
        assert_eq!(
            sheet.cell_format_summary(Pos { x: 1, y: 1 }, false),
            CellFormatSummary {
                bold: Some(true),
                italic: None,
                text_color: None,
                fill_color: None,
                commas: None,
            }
        );
        assert_eq!(
            sheet.display_value(Pos { x: 3, y: 2 }),
            Some(CellValue::Number(BigDecimal::from(12)))
        );
        assert_eq!(
            sheet.cell_format_summary(Pos { x: 3, y: 2 }, false),
            CellFormatSummary {
                bold: None,
                italic: Some(true),
                text_color: None,
                fill_color: None,
                commas: None,
            }
        );

        // use to create output for test_paste_from_quadratic_clipboard()
        print!("{}", clipboard.1);
    }

    #[test]
    fn test_copy_code_to_clipboard() {
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];

        set_formula_code_cell(&mut gc, sheet_id, "1 + 1", 1, 1);

        assert_eq!(gc.undo_stack.len(), 1);
        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.display_value(Pos { x: 1, y: 1 }),
            Some(CellValue::Number(BigDecimal::from(2)))
        );

        let selection = Selection::rect(
            Rect {
                min: Pos { x: 1, y: 1 },
                max: Pos { x: 1, y: 1 },
            },
            sheet_id,
        );
        let clipboard = sheet.copy_to_clipboard(&selection).unwrap();

        // paste using html on a new grid controller
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];

        // ensure the grid controller is empty
        assert_eq!(gc.undo_stack.len(), 0);

        gc.paste_from_clipboard(
            Selection::pos(0, 0, sheet_id),
            None,
            Some(clipboard.1.clone()),
            PasteSpecial::None,
            None,
        );
        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.display_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Number(BigDecimal::from(2)))
        );
        gc.undo(None);
        let sheet = gc.sheet(sheet_id);
        assert_eq!(sheet.display_value(Pos { x: 0, y: 0 }), None);

        // prepare a cell to be overwritten
        set_formula_code_cell(&mut gc, sheet_id, "2 + 2", 0, 0);

        assert_eq!(gc.undo_stack.len(), 1);

        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.display_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Number(BigDecimal::from(4)))
        );

        gc.paste_from_clipboard(
            Selection::pos(0, 0, sheet_id),
            Some(String::from("")),
            Some(clipboard.1),
            PasteSpecial::None,
            None,
        );
        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.display_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Number(BigDecimal::from(2)))
        );

        assert_eq!(gc.undo_stack.len(), 2);

        // undo to original code cell value
        gc.undo(None);

        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.display_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Number(BigDecimal::from(4)))
        );

        assert_eq!(gc.undo_stack.len(), 1);

        // empty code cell
        gc.undo(None);
        let sheet = gc.sheet(sheet_id);
        assert_eq!(sheet.display_value(Pos { x: 0, y: 0 }), None);

        assert_eq!(gc.undo_stack.len(), 0);
    }

    #[test]
    fn test_copy_code_to_clipboard_with_array_output() {
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];

        set_formula_code_cell(&mut gc, sheet_id, "{1, 2, 3}", 1, 1);

        assert_eq!(gc.undo_stack.len(), 1);
        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.display_value(Pos { x: 1, y: 1 }),
            Some(CellValue::Number(BigDecimal::from(1)))
        );
        assert_eq!(
            sheet.display_value(Pos { x: 2, y: 1 }),
            Some(CellValue::Number(BigDecimal::from(2)))
        );
        assert_eq!(
            sheet.display_value(Pos { x: 3, y: 1 }),
            Some(CellValue::Number(BigDecimal::from(3)))
        );
        let selection = Selection::rect(
            Rect::new_span(Pos { x: 1, y: 1 }, Pos { x: 3, y: 1 }),
            sheet_id,
        );
        let clipboard = sheet.copy_to_clipboard(&selection).unwrap();

        // paste using html on a new grid controller
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];

        // ensure the grid controller is empty
        assert_eq!(gc.undo_stack.len(), 0);

        gc.paste_from_clipboard(
            Selection::pos(0, 0, sheet_id),
            None,
            Some(clipboard.1.clone()),
            PasteSpecial::None,
            None,
        );
        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.display_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Number(BigDecimal::from(1)))
        );
        assert_eq!(
            sheet.display_value(Pos { x: 1, y: 0 }),
            Some(CellValue::Number(BigDecimal::from(2)))
        );
        assert_eq!(
            sheet.display_value(Pos { x: 2, y: 0 }),
            Some(CellValue::Number(BigDecimal::from(3)))
        );

        gc.undo(None);
        let sheet = gc.sheet(sheet_id);
        assert_eq!(sheet.display_value(Pos { x: 0, y: 0 }), None);
        assert_eq!(sheet.display_value(Pos { x: 1, y: 0 }), None);
        assert_eq!(sheet.display_value(Pos { x: 2, y: 0 }), None);
    }

    #[test]
    fn test_copy_borders_to_clipboard() {
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];
        let sheet = gc.sheet_mut(sheet_id);

        set_borders(sheet);

        let selection = Selection::rect(
            Rect::new_span(Pos { x: 0, y: 0 }, Pos { x: 0, y: 0 }),
            sheet_id,
        );
        let clipboard = sheet.copy_to_clipboard(&selection).unwrap();

        gc.paste_from_clipboard(
            Selection::pos(3, 3, sheet_id),
            Some(String::from("")),
            Some(clipboard.1),
            PasteSpecial::None,
            None,
        );

        let borders = gc
            .sheet(sheet_id)
            .borders()
            .per_cell
            .borders
            .iter()
            .collect::<Vec<_>>();

        // compare the border info stored in the block's content
        assert_eq!(
            borders[0].1.blocks().next().unwrap().content,
            borders[1].1.blocks().next().unwrap().content
        );
    }

    #[test]
    fn test_copy_borders_inside() {
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];
        let sheet = gc.sheet_mut(sheet_id);

        let selection = vec![BorderSelection::Outer];
        let style = BorderStyle {
            color: Rgba::color_from_str("#000000").unwrap(),
            line: CellBorderLine::Line1,
        };
        let rect = Rect::new_span(Pos { x: 0, y: 0 }, Pos { x: 4, y: 4 });
        let borders = generate_borders(sheet, &rect, selection, Some(style));
        set_rect_borders(sheet, &rect, borders);

        // weird: can't test them by comparing arrays since the order is seemingly random
        let borders = sheet.render_borders();
        assert!(borders.horizontal.iter().any(|border| {
            border.x == 0
                && border.y == 0
                && border.w == Some(5)
                && border.h.is_none()
                && border.style == style
        }));
        assert!(borders.horizontal.iter().any(|border| {
            border.x == 0
                && border.y == 5
                && border.w == Some(5)
                && border.h.is_none()
                && border.style == style
        }));
        assert!(borders.vertical.iter().any(|border| {
            border.x == 0
                && border.y == 0
                && border.w.is_none()
                && border.h == Some(5)
                && border.style == style
        }));

        assert!(borders.vertical.iter().any(|border| {
            border.x == 5
                && border.y == 0
                && border.w.is_none()
                && border.h == Some(5)
                && border.style == style
        }));

        let (_, html) = sheet
            .copy_to_clipboard(&Selection::rect(
                Rect::new_span(Pos { x: 0, y: 0 }, Pos { x: 4, y: 4 }),
                sheet_id,
            ))
            .unwrap();
        gc.paste_from_clipboard(
            Selection::pos(0, 10, sheet_id),
            None,
            Some(html),
            PasteSpecial::None,
            None,
        );

        let sheet = gc.sheet_mut(sheet_id);
        let borders = sheet.render_borders();
        assert!(borders.horizontal.iter().any(|border| {
            border.x == 0
                && border.y == 10
                && border.w == Some(5)
                && border.h.is_none()
                && border.style == style
        }));
        assert!(borders.horizontal.iter().any(|border| {
            border.x == 0
                && border.y == 15
                && border.w == Some(5)
                && border.h.is_none()
                && border.style == style
        }));
        assert!(borders.vertical.iter().any(|border| {
            border.x == 0
                && border.y == 10
                && border.w.is_none()
                && border.h == Some(5)
                && border.style == style
        }));
        assert!(borders.vertical.iter().any(|border| {
            border.x == 5
                && border.y == 10
                && border.w.is_none()
                && border.h == Some(5)
                && border.style == style
        }));
    }

    #[test]
    fn test_paste_from_quadratic_clipboard() {
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];

        // see line ~357 for the output (`print!("{}", clipboard.1);`)
        let pasted_output = String::from(
            r#"<table data-quadratic="&#x7B;&quot;w&quot;&#x3A;4&#x2C;&quot;h&quot;&#x3A;4&#x2C;&quot;cells&quot;&#x3A;&#x7B;&quot;columns&quot;&#x3A;&#x5B;&#x7B;&#x7D;&#x2C;&#x7B;&quot;1&quot;&#x3A;&#x7B;&quot;type&quot;&#x3A;&quot;text&quot;&#x2C;&quot;value&quot;&#x3A;&quot;1&#x2C;&#x20;1&quot;&#x7D;&#x7D;&#x2C;&#x7B;&#x7D;&#x2C;&#x7B;&quot;2&quot;&#x3A;&#x7B;&quot;type&quot;&#x3A;&quot;number&quot;&#x2C;&quot;value&quot;&#x3A;&quot;12&quot;&#x7D;&#x7D;&#x5D;&#x2C;&quot;w&quot;&#x3A;4&#x2C;&quot;h&quot;&#x3A;4&#x7D;&#x2C;&quot;values&quot;&#x3A;&#x7B;&quot;columns&quot;&#x3A;&#x5B;&#x7B;&#x7D;&#x2C;&#x7B;&quot;1&quot;&#x3A;&#x7B;&quot;type&quot;&#x3A;&quot;text&quot;&#x2C;&quot;value&quot;&#x3A;&quot;1&#x2C;&#x20;1&quot;&#x7D;&#x7D;&#x2C;&#x7B;&#x7D;&#x2C;&#x7B;&quot;2&quot;&#x3A;&#x7B;&quot;type&quot;&#x3A;&quot;number&quot;&#x2C;&quot;value&quot;&#x3A;&quot;12&quot;&#x7D;&#x7D;&#x5D;&#x2C;&quot;w&quot;&#x3A;4&#x2C;&quot;h&quot;&#x3A;4&#x7D;&#x2C;&quot;formats&quot;&#x3A;&#x5B;&#x7B;&quot;Align&quot;&#x3A;&#x5B;&#x5B;null&#x2C;16&#x5D;&#x5D;&#x7D;&#x2C;&#x7B;&quot;Wrap&quot;&#x3A;&#x5B;&#x5B;null&#x2C;16&#x5D;&#x5D;&#x7D;&#x2C;&#x7B;&quot;NumericFormat&quot;&#x3A;&#x5B;&#x5B;null&#x2C;16&#x5D;&#x5D;&#x7D;&#x2C;&#x7B;&quot;NumericDecimals&quot;&#x3A;&#x5B;&#x5B;null&#x2C;16&#x5D;&#x5D;&#x7D;&#x2C;&#x7B;&quot;NumericCommas&quot;&#x3A;&#x5B;&#x5B;null&#x2C;16&#x5D;&#x5D;&#x7D;&#x2C;&#x7B;&quot;Bold&quot;&#x3A;&#x5B;&#x5B;null&#x2C;5&#x5D;&#x2C;&#x5B;true&#x2C;1&#x5D;&#x2C;&#x5B;null&#x2C;10&#x5D;&#x5D;&#x7D;&#x2C;&#x7B;&quot;Italic&quot;&#x3A;&#x5B;&#x5B;null&#x2C;11&#x5D;&#x2C;&#x5B;true&#x2C;1&#x5D;&#x2C;&#x5B;null&#x2C;4&#x5D;&#x5D;&#x7D;&#x2C;&#x7B;&quot;TextColor&quot;&#x3A;&#x5B;&#x5B;null&#x2C;16&#x5D;&#x5D;&#x7D;&#x2C;&#x7B;&quot;FillColor&quot;&#x3A;&#x5B;&#x5B;null&#x2C;16&#x5D;&#x5D;&#x7D;&#x5D;&#x2C;&quot;borders&quot;&#x3A;&#x5B;&#x5B;0&#x2C;0&#x2C;null&#x5D;&#x2C;&#x5B;0&#x2C;1&#x2C;null&#x5D;&#x2C;&#x5B;0&#x2C;2&#x2C;null&#x5D;&#x2C;&#x5B;0&#x2C;3&#x2C;null&#x5D;&#x2C;&#x5B;1&#x2C;0&#x2C;null&#x5D;&#x2C;&#x5B;1&#x2C;1&#x2C;null&#x5D;&#x2C;&#x5B;1&#x2C;2&#x2C;null&#x5D;&#x2C;&#x5B;1&#x2C;3&#x2C;null&#x5D;&#x2C;&#x5B;2&#x2C;0&#x2C;null&#x5D;&#x2C;&#x5B;2&#x2C;1&#x2C;null&#x5D;&#x2C;&#x5B;2&#x2C;2&#x2C;null&#x5D;&#x2C;&#x5B;2&#x2C;3&#x2C;null&#x5D;&#x2C;&#x5B;3&#x2C;0&#x2C;null&#x5D;&#x2C;&#x5B;3&#x2C;1&#x2C;null&#x5D;&#x2C;&#x5B;3&#x2C;2&#x2C;null&#x5D;&#x2C;&#x5B;3&#x2C;3&#x2C;null&#x5D;&#x5D;&#x2C;&quot;origin&quot;&#x3A;&#x7B;&quot;x&quot;&#x3A;0&#x2C;&quot;y&quot;&#x3A;0&#x2C;&quot;column&quot;&#x3A;null&#x2C;&quot;row&quot;&#x3A;null&#x2C;&quot;all&quot;&#x3A;null&#x7D;&#x2C;&quot;selection&quot;&#x3A;&#x7B;&quot;sheet&#x5F;id&quot;&#x3A;&#x7B;&quot;id&quot;&#x3A;&quot;05d744eb&#x2D;18a6&#x2D;4f98&#x2D;95c8&#x2D;85b5c6e41930&quot;&#x7D;&#x2C;&quot;x&quot;&#x3A;0&#x2C;&quot;y&quot;&#x3A;0&#x2C;&quot;rects&quot;&#x3A;&#x5B;&#x7B;&quot;min&quot;&#x3A;&#x7B;&quot;x&quot;&#x3A;0&#x2C;&quot;y&quot;&#x3A;0&#x7D;&#x2C;&quot;max&quot;&#x3A;&#x7B;&quot;x&quot;&#x3A;3&#x2C;&quot;y&quot;&#x3A;3&#x7D;&#x7D;&#x5D;&#x2C;&quot;rows&quot;&#x3A;null&#x2C;&quot;columns&quot;&#x3A;null&#x2C;&quot;all&quot;&#x3A;false&#x7D;&#x7D;"><tbody><tr><td ></td><td ></td><td ></td><td ></tr><tr><td ></td><td style="font-weight:bold;">1, 1</td><td ></td><td ></tr><tr><td ></td><td ></td><td ></td><td style="font-style:italic;">12</tr><tr><td ></td><td ></td><td ></td><td ></td></tr></tbody></table>"#,
        );

        gc.paste_from_clipboard(
            Selection::pos(1, 2, sheet_id),
            None,
            Some(pasted_output),
            PasteSpecial::None,
            None,
        );

        let sheet = gc.sheet(sheet_id);
        let cell11 = sheet.display_value(Pos { x: 2, y: 3 });
        assert_eq!(cell11.unwrap(), CellValue::Text(String::from("1, 1")));
        let cell21 = sheet.display_value(Pos { x: 4, y: 4 });
        assert_eq!(cell21.unwrap(), CellValue::Number(BigDecimal::from(12)));
    }

    // | 1 | A0           |
    // | 2 | [paste here] |
    //
    // paste the code cell (0,1) => A0 from the clipboard to (1,1),
    // expect value to change to 2
    #[test]
    fn test_paste_relative_code_from_quadratic_clipboard() {
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];
        let src_pos: Pos = (3, 2).into();

        set_formula_code_cell(&mut gc, sheet_id, "SUM( C2)", src_pos.x, src_pos.y);
        set_cell_value(&mut gc, sheet_id, "1", 2, 2);
        set_cell_value(&mut gc, sheet_id, "2", 2, 3);
        set_cell_value(&mut gc, sheet_id, "3", 2, 4);

        // generate the html from the values above
        let sheet = gc.sheet(sheet_id);
        let (_, html) = sheet
            .copy_to_clipboard(&Selection::rect(src_pos.into(), sheet_id))
            .unwrap();

        let get_value = |gc: &GridController, x, y| {
            let sheet = gc.sheet(sheet_id);
            let cell_value = sheet.cell_value(Pos { x, y });
            let display_value = sheet.display_value(Pos { x, y });
            (cell_value, display_value)
        };

        let assert_code_cell =
            |gc: &mut GridController, dest_pos: SheetPos, code: &str, value: i32| {
                gc.paste_from_clipboard(
                    Selection::pos(dest_pos.x, dest_pos.y, sheet_id),
                    None,
                    Some(html.clone()),
                    PasteSpecial::None,
                    None,
                );

                let cell_value = get_value(gc, dest_pos.x, dest_pos.y);
                let expected_cell_value = Some(CellValue::Code(CodeCellValue {
                    language: CodeCellLanguage::Formula,
                    code: code.into(),
                }));
                let expected_display_value = Some(CellValue::Number(BigDecimal::from(value)));

                assert_eq!(cell_value, (expected_cell_value, expected_display_value));
            };

        // paste code cell (3,2) from the clipboard to (3,3)
        let dest_pos: SheetPos = (3, 3, sheet_id).into();
        assert_code_cell(&mut gc, dest_pos, "SUM( C3)", 2);

        // paste code cell (3,2) from the clipboard to (3,4)
        let dest_pos: SheetPos = (3, 4, sheet_id).into();
        assert_code_cell(&mut gc, dest_pos, "SUM( C4)", 3);
    }

    #[test]
    fn paste_special_values() {
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];

        set_formula_code_cell(&mut gc, sheet_id, "{1, 2, 3}", 1, 1);

        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.display_value(Pos { x: 1, y: 1 }),
            Some(CellValue::Number(BigDecimal::from(1)))
        );
        assert_eq!(
            sheet.display_value(Pos { x: 3, y: 1 }),
            Some(CellValue::Number(BigDecimal::from(3)))
        );

        let sheet = gc.sheet(sheet_id);
        let (plain, html) = sheet
            .copy_to_clipboard(&Selection::rect(
                Rect {
                    min: Pos { x: 1, y: 1 },
                    max: Pos { x: 3, y: 1 },
                },
                sheet_id,
            ))
            .unwrap();

        gc.paste_from_clipboard(
            Selection::pos(0, 2, sheet_id),
            Some(plain),
            Some(html),
            PasteSpecial::Values,
            None,
        );

        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.cell_value(Pos { x: 0, y: 2 }),
            Some(CellValue::Number(BigDecimal::from(1)))
        );
        assert_eq!(
            sheet.cell_value(Pos { x: 1, y: 2 }),
            Some(CellValue::Number(BigDecimal::from(2)))
        );
        assert_eq!(
            sheet.cell_value(Pos { x: 2, y: 2 }),
            Some(CellValue::Number(BigDecimal::from(3)))
        );
    }

    #[test]
    fn paste_special_formats() {
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];

        set_cell_value(&mut gc, sheet_id, "1", 1, 1);
        gc.set_cell_bold(
            SheetRect {
                min: Pos { x: 1, y: 1 },
                max: Pos { x: 1, y: 1 },
                sheet_id,
            },
            Some(true),
            None,
        );

        set_cell_value(&mut gc, sheet_id, "12", 2, 2);
        gc.set_cell_italic(
            SheetRect {
                min: Pos { x: 2, y: 2 },
                max: Pos { x: 2, y: 2 },
                sheet_id,
            },
            Some(true),
            None,
        );

        let selection = Selection::rect(
            Rect {
                min: Pos { x: 0, y: 0 },
                max: Pos { x: 2, y: 2 },
            },
            sheet_id,
        );

        let sheet = gc.sheet(sheet_id);
        let (plain_text, html) = sheet.copy_to_clipboard(&selection).unwrap();
        assert_eq!(plain_text, String::from("\t\t\n\t1\t\n\t\t12"));

        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];
        gc.paste_from_clipboard(
            Selection::pos(0, 0, sheet_id),
            Some(plain_text),
            Some(html),
            PasteSpecial::Formats,
            None,
        );
        let sheet = gc.sheet(sheet_id);
        assert_eq!(sheet.display_value(Pos { x: 1, y: 1 }), None);
        assert_eq!(sheet.display_value(Pos { x: 2, y: 2 }), None);
        assert_eq!(
            sheet.cell_format_summary(Pos { x: 1, y: 1 }, false),
            CellFormatSummary {
                bold: Some(true),
                italic: None,
                text_color: None,
                fill_color: None,
                commas: None,
            }
        );
        assert_eq!(
            sheet.cell_format_summary(Pos { x: 2, y: 2 }, false),
            CellFormatSummary {
                bold: None,
                italic: Some(true),
                text_color: None,
                fill_color: None,
                commas: None,
            }
        );
    }

    #[test]
    fn copy_part_of_code_run() {
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];

        set_formula_code_cell(&mut gc, sheet_id, "{1, 2, 3; 4, 5, 6}", 1, 1);

        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.display_value(Pos { x: 1, y: 1 }),
            Some(CellValue::Number(BigDecimal::from(1)))
        );

        // don't copy the origin point
        let selection = Selection::rect(
            Rect::new_span(Pos { x: 2, y: 1 }, Pos { x: 3, y: 2 }),
            sheet_id,
        );
        let sheet = gc.sheet(sheet_id);
        let clipboard = sheet.copy_to_clipboard(&selection).unwrap();

        // paste using html on a new grid controller
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];

        // ensure the grid controller is empty
        assert_eq!(gc.undo_stack.len(), 0);

        gc.paste_from_clipboard(
            Selection::pos(0, 0, sheet_id),
            Some(clipboard.0),
            Some(clipboard.1),
            PasteSpecial::None,
            None,
        );
        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.display_value(Pos { x: 0, y: 0 }),
            Some(CellValue::Number(BigDecimal::from(2)))
        );

        gc.undo(None);
        let sheet = gc.sheet(sheet_id);
        assert_eq!(sheet.display_value(Pos { x: 0, y: 0 }), None);
    }

    #[test]
    fn move_cells() {
        let mut gc = GridController::default();
        let sheet_id = gc.sheet_ids()[0];

        set_formula_code_cell(&mut gc, sheet_id, "{1, 2, 3; 4, 5, 6}", 0, 0);
        set_cell_value(&mut gc, sheet_id, "100", 0, 2);

        gc.move_cells(
            SheetRect::new_pos_span(Pos { x: 0, y: 0 }, Pos { x: 3, y: 2 }, sheet_id),
            (10, 10, sheet_id).into(),
            None,
        );

        let sheet = gc.sheet(sheet_id);
        assert_eq!(
            sheet.display_value((10, 10).into()),
            Some(CellValue::Number(BigDecimal::from(1)))
        );
        assert_eq!(
            sheet.display_value((10, 12).into()),
            Some(CellValue::Number(BigDecimal::from(100)))
        );
        assert_eq!(sheet.display_value((0, 0).into()), None);
        assert_eq!(sheet.display_value((0, 2).into()), None);

        gc.undo(None);

        let sheet = gc.sheet(sheet_id);
        assert_eq!(sheet.display_value((10, 10).into()), None);
        assert_eq!(sheet.display_value((10, 12).into()), None);
        assert_eq!(
            sheet.display_value((0, 0).into()),
            Some(CellValue::Number(BigDecimal::from(1)))
        );
        assert_eq!(
            sheet.display_value((0, 2).into()),
            Some(CellValue::Number(BigDecimal::from(100)))
        );
    }
}
