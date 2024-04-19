import { events } from '@/events/events';
import { sheets } from '@/grid/controller/Sheets';
import { colors } from '@/theme/colors';
import { Graphics } from 'pixi.js';
import { pixiApp } from '../pixiApp/PixiApp';
import { SheetPosTS } from '../types/size';

export class CellsSearch extends Graphics {
  private sheetId: string;

  constructor(sheetId: string) {
    super();
    this.sheetId = sheetId;
    events.on('search', this.handleSearch);
  }

  private handleSearch = (found?: SheetPosTS[], current?: number) => {
    this.clear();
    if (found?.length) {
      found.forEach((cell: SheetPosTS, index: number) => {
        const { x, y, sheetId } = cell;
        if (this.sheetId === sheetId) {
          const sheet = sheets.getById(sheetId);
          if (!sheet) throw new Error('Expected sheet to be defined in CellsSearch.handleSearch');
          const offsets = sheet.getCellOffsets(Number(x), Number(y));
          this.beginFill(colors.searchCell, current === index ? 1 : 0.1);
          this.drawRect(offsets.x, offsets.y, offsets.width, offsets.height);
          this.endFill();
        }
      });
    }
    if (sheets.sheet.id === this.sheetId) {
      pixiApp.setViewportDirty();
    }
  };
}
