import { Point } from 'pixi.js';
import { IS_READONLY_MODE } from '../../../constants/app';
import { Sheet } from '../../../grid/sheet/Sheet';
import { PixiApp } from '../../pixiApp/PixiApp';
import { PanMode } from '../../pixiApp/PixiAppSettings';
import { doubleClickCell } from './doubleClickCell';
import { DOUBLE_CLICK_TIME } from './pointerUtils';

const MINIMUM_MOVE_POSITION = 5;

export class PointerDown {
  private app: PixiApp;
  active = false;

  private positionRaw?: Point;
  private position?: Point;
  private previousPosition?: { originPosition: Point; terminalPosition: Point };
  private pointerMoved = false;
  private doubleClickTimeout?: number;

  // flag that ensures that if pointerUp triggers during setTimeout, pointerUp is still called (see below)
  private afterShowInput?: boolean;

  constructor(app: PixiApp) {
    this.app = app;
  }

  get sheet(): Sheet {
    return this.app.sheet;
  }

  pointerDown(world: Point, event: PointerEvent): void {
    if (IS_READONLY_MODE) return;
    if (this.app.settings.panMode !== PanMode.Disabled) return;

    const cursor = this.app.sheet.cursor;

    // note: directly call this.app.settings instead of locally defining it here; otherwise it dereferences this

    // this is a hack to ensure CellInput properly closes and updates before the cursor moves positions
    if (this.app.settings.input.show) {
      this.afterShowInput = true;
      setTimeout(() => {
        this.pointerDown(world, event);
        this.afterShowInput = false;
      }, 0);
      return;
    }

    const { gridOffsets } = this.sheet;

    this.positionRaw = world;
    const { column, row } = gridOffsets.getRowColumnFromWorld(world.x, world.y);

    const rightClick = event.button === 2 || (event.button === 0 && event.ctrlKey);

    // If right click and we have a multi cell selection.
    // If the user has clicked inside the selection.
    if (rightClick && cursor.multiCursor) {
      if (
        column >= cursor.multiCursor.originPosition.x &&
        column <= cursor.multiCursor.terminalPosition.x &&
        row >= cursor.multiCursor.originPosition.y &&
        row <= cursor.multiCursor.terminalPosition.y
      )
        // Ignore this click. User is accessing the RightClickMenu.
        return;
    }

    if (this.doubleClickTimeout) {
      window.clearTimeout(this.doubleClickTimeout);
      this.doubleClickTimeout = undefined;
      if (
        this.previousPosition &&
        column === this.previousPosition.originPosition.x &&
        row === this.previousPosition.originPosition.y
      ) {
        // ignore right click
        if (rightClick) {
          return;
        }
        const code = this.sheet.getCodeValue(column, row);
        const cell = this.sheet.getRenderCell(column, row);
        doubleClickCell({ column, row, code, cell });
        this.active = false;
        event.preventDefault();
        return;
      }
    }

    // select cells between pressed and cursor position
    if (event.shiftKey) {
      const { column, row } = gridOffsets.getRowColumnFromWorld(world.x, world.y);
      const cursorPosition = cursor.cursorPosition;
      if (column !== cursorPosition.x || row !== cursorPosition.y) {
        // make origin top left, and terminal bottom right
        const originX = cursorPosition.x < column ? cursorPosition.x : column;
        const originY = cursorPosition.y < row ? cursorPosition.y : row;
        const termX = cursorPosition.x > column ? cursorPosition.x : column;
        const termY = cursorPosition.y > row ? cursorPosition.y : row;

        cursor.changePosition({
          keyboardMovePosition: { x: column, y: row },
          multiCursor: {
            originPosition: new Point(originX, originY),
            terminalPosition: new Point(termX, termY),
          },
        });
      }
      return;
    }

    this.active = true;
    this.position = new Point(column, row);

    const previousPosition = {
      originPosition: new Point(column, row),
      terminalPosition: new Point(column, row),
    };

    // Keep track of multiCursor previous position
    this.previousPosition = previousPosition;

    // Move cursor to mouse down position
    // For single click, hide multiCursor
    cursor.changePosition({
      keyboardMovePosition: { x: column, y: row },
      cursorPosition: { x: column, y: row },
    });
    this.pointerMoved = false;
  }

  pointerMove(world: Point): void {
    if (this.app.settings.panMode !== PanMode.Disabled) return;

    if (!this.active) return;

    const { viewport } = this.app;
    const { gridOffsets } = this.sheet;

    // for determining if double click
    if (!this.pointerMoved && this.doubleClickTimeout && this.positionRaw) {
      if (
        Math.abs(this.positionRaw.x - world.x) + Math.abs(this.positionRaw.y - world.y) >
        MINIMUM_MOVE_POSITION / viewport.scale.x
      ) {
        this.pointerMoved = true;
        this.clearDoubleClick();
      }
    }

    // cursor intersects bottom-corner indicator (disabled for now)
    if (!this.active || !this.position || !this.previousPosition || !this.positionRaw) {
      return;
    }

    // calculate mouse move position
    const { column, row } = gridOffsets.getRowColumnFromWorld(world.x, world.y);

    // cursor start and end in the same cell
    if (column === this.position.x && row === this.position.y) {
      // hide multi cursor when only selecting one cell
      this.sheet.cursor.changePosition({
        keyboardMovePosition: { x: this.position.x, y: this.position.y },
        cursorPosition: { x: this.position.x, y: this.position.y },
      });
      this.app.settings.changeInput(false);
    } else {
      // cursor origin and terminal are not in the same cell

      // make origin top left, and terminal bottom right
      const originX = this.position.x < column ? this.position.x : column;
      const originY = this.position.y < row ? this.position.y : row;
      const termX = this.position.x > column ? this.position.x : column;
      const termY = this.position.y > row ? this.position.y : row;

      // determine if the cursor has moved from the previous event
      const hasMoved = !(
        this.previousPosition.originPosition.x === originX &&
        this.previousPosition.originPosition.y === originY &&
        this.previousPosition.terminalPosition.x === termX &&
        this.previousPosition.terminalPosition.y === termY
      );

      // only set state if changed
      // this reduces the number of hooks fired
      if (hasMoved) {
        // update multiCursor
        this.sheet.cursor.changePosition({
          keyboardMovePosition: { x: column, y: row },
          cursorPosition: { x: this.position.x, y: this.position.y },
          multiCursor: {
            originPosition: { x: originX, y: originY },
            terminalPosition: { x: termX, y: termY },
          },
        });
        this.app.settings.changeInput(false);

        // update previousPosition
        this.previousPosition = {
          originPosition: new Point(originX, originY),
          terminalPosition: new Point(termX, termY),
        };
      }
    }
  }

  pointerUp(): void {
    if (this.afterShowInput) {
      window.setTimeout(() => this.pointerUp(), 0);
      this.afterShowInput = false;
      return;
    }
    if (this.active) {
      if (!this.pointerMoved) {
        this.doubleClickTimeout = window.setTimeout(() => (this.doubleClickTimeout = undefined), DOUBLE_CLICK_TIME);
      }
      this.active = false;
    }
  }

  private clearDoubleClick(): void {
    if (this.doubleClickTimeout) {
      window.clearTimeout(this.doubleClickTimeout);
      this.doubleClickTimeout = undefined;
    }
  }

  destroy(): void {
    this.clearDoubleClick();
  }
}
