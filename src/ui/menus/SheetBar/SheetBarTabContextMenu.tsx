import { ControlledMenu, MenuDivider, MenuItem, SubMenu } from '@szhsin/react-menu';
import { useState } from 'react';
import { ColorResult } from 'react-color';
import { updateSheet } from '../../../grid/actions/sheetsAction';
import { SheetController } from '../../../grid/controller/SheetController';
import { convertReactColorToString } from '../../../helpers/convertColor';
import { focusGrid } from '../../../helpers/focusGrid';
import { generateKeyBetween } from '../../../utils/fractionalIndexing';
import { QColorPicker } from '../../components/qColorPicker';
import { ConfirmDeleteSheet } from './ConfirmDeleteSheet';

interface Props {
  sheetController: SheetController;
  contextMenu?: { x: number; y: number; id: string; name: string };
  handleRename: () => void;
  handleClose: () => void;
}

export const SheetBarTabContextMenu = (props: Props): JSX.Element => {
  const { sheetController, contextMenu, handleClose, handleRename } = props;
  const [confirmDelete, setConfirmDelete] = useState<{ id: string; name: string } | undefined>();
  const [lastName, setLastName] = useState<string | undefined>();

  return (
    <>
      <ControlledMenu
        className="sheet-bar-context-menu"
        state={!!contextMenu ? 'open' : 'closed'}
        onClose={handleClose}
        anchorPoint={contextMenu ? { x: contextMenu?.x, y: contextMenu?.y } : undefined}
      >
        <MenuItem onClick={handleRename}>
          <b>Rename</b>
        </MenuItem>
        <MenuItem
          onClick={handleClose}
          onClickCapture={() => {
            sheetController.sheets.duplicate();
            focusGrid();
          }}
        >
          Duplicate
        </MenuItem>
        <MenuItem
          onClick={() => {
            if (!contextMenu) return;
            setConfirmDelete({ ...contextMenu });
            setLastName(confirmDelete?.name);
            handleClose();
          }}
        >
          Delete
        </MenuItem>
        <SubMenu label="Change Color">
          <QColorPicker
            onChangeComplete={(change: ColorResult) => {
              const color = convertReactColorToString(change);
              if (contextMenu) {
                sheetController.sheet.color = color;
                focusGrid();
              }
              handleClose();
            }}
            onClear={() => {
              if (contextMenu) {
                sheetController.sheet.color = undefined;
                focusGrid();
              }
              handleClose();
            }}
          />
        </SubMenu>
        <MenuDivider />
        <MenuItem
          disabled={sheetController.sheets.getFirst().id === contextMenu?.id}
          onClick={() => {
            if (contextMenu) {
              const sheet = sheetController.sheet;
              const previous = sheetController.sheets.getPrevious(sheet.order)?.order ?? null;
              const previousSecond = previous ? sheetController.sheets.getPrevious(previous)?.order ?? null : null;
              const order = generateKeyBetween(previousSecond, previous);
              updateSheet({
                sheetController,
                sheet: sheetController.sheet,
                order,
                create_transaction: true,
              });
              focusGrid();
            }
            handleClose();
          }}
        >
          Move Left
        </MenuItem>
        <MenuItem
          disabled={sheetController.sheets.getLast().id === contextMenu?.id}
          onClick={() => {
            if (contextMenu) {
              const sheet = sheetController.sheet;
              const next = sheetController.sheets.getNext(sheet.order)?.order ?? null;
              const nextSecond = next ? sheetController.sheets.getNext(next)?.order ?? null : null;
              const order = generateKeyBetween(next, nextSecond);
              updateSheet({
                sheetController,
                sheet: sheetController.sheet,
                order,
                create_transaction: true,
              });
              focusGrid();
            }
            handleClose();
          }}
        >
          Move Right
        </MenuItem>
      </ControlledMenu>
      <ConfirmDeleteSheet
        sheetController={sheetController}
        lastName={lastName}
        confirmDelete={confirmDelete}
        handleClose={() => {
          setConfirmDelete(undefined);
          window.setTimeout(focusGrid, 0);
        }}
      />
    </>
  );
};
