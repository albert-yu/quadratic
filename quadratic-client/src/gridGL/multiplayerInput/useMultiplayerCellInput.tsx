// Displays when a multiplayer user is editing a cell

import { useEffect, useState } from 'react';
import { Coordinate } from '../types/size';

export interface MultiplayerCell {
  sessionId: string;
  sheetId: string;
  cell: Coordinate;
  text?: string;
  color: string;
  fillColor?: string;
  bold: boolean;
  italic: boolean;
  cursor: number;
  playerColor: string;
}

export const useMultiplayerCellInput = (): MultiplayerCell[] => {
  const [multiplayerCellInput, setMultiplayerCellInput] = useState<MultiplayerCell[]>([]);
  useEffect(() => {
    const updateMultiplayerCellEdit = (e: any) => {
      const multiplayerCell = e.detail as MultiplayerCell;
      setMultiplayerCellInput((prev) => {
        const found = prev.findIndex((prev) => prev.sessionId === multiplayerCell.sessionId);
        if (multiplayerCell && found === -1) {
          return [...prev, multiplayerCell];
        }
        return prev.map((cell, index) => {
          if (index === found) return multiplayerCell;
          return cell;
        });
      });
    };
    window.addEventListener('multiplayer-cell-edit', updateMultiplayerCellEdit);
    return () => window.removeEventListener('multiplayer-cell-edit', updateMultiplayerCellEdit);
  }, []);

  // only return users that are actively editing (ie, text !== undefined)
  return multiplayerCellInput.filter((cell) => cell.text !== undefined);
};
