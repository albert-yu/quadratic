import { JsCodeCell, JsRenderCodeCell, JsRenderFill } from '@/quadratic-core/types';

export interface ClientCoreLoad {
  type: 'clientCoreLoad';
  url: string;
  version: string;
  sequenceNumber: number;
  id: number;
}

export interface SheetMetadata {
  offsets: string;
  bounds?: { x: number; y: number; width: number; height: number };
  boundsNoFormatting?: { x: number; y: number; width: number; height: number };
  name: string;
  order: string;
  color?: string;
}

export interface GridMetadata {
  undo: boolean;
  redo: boolean;
  sheets: Record<string, SheetMetadata>;
}

export interface CoreClientLoad {
  type: 'coreClientLoad';
  id: number;
  metadata: GridMetadata;
}

export interface ClientCoreGetCodeCell {
  type: 'clientCoreGetCodeCell';
  sheetId: string;
  x: number;
  y: number;
  id: number;
}

export interface CoreClientGetCodeCell {
  type: 'coreClientGetCodeCell';
  cell: JsCodeCell | undefined;
  id: number;
}

export interface ClientCoreGetAllRenderFills {
  type: 'clientCoreGetAllRenderFills';
  sheetId: string;
  id: number;
}

export interface CoreClientGetAllRenderFills {
  type: 'coreClientGetAllRenderFills';
  fills: JsRenderFill[];
  id: number;
}

export interface ClientCoreGetRenderCodeCells {
  type: 'clientCoreGetRenderCodeCells';
  sheetId: string;
  id: number;
}

export interface CoreClientGetRenderCodeCells {
  type: 'coreClientGetRenderCodeCells';
  codeCells: JsRenderCodeCell[];
  id: number;
}

export type ClientCoreMessage =
  | ClientCoreLoad
  | ClientCoreGetCodeCell
  | ClientCoreGetAllRenderFills
  | ClientCoreGetRenderCodeCells;

export type CoreClientMessage =
  | CoreClientLoad
  | CoreClientGetCodeCell
  | CoreClientGetAllRenderFills
  | CoreClientGetRenderCodeCells;
