import TopBar from '../ui/menus/TopBar';
import CellTypeMenu from '../ui/menus/CellTypeMenu/';
import CodeEditor from '../ui/menus/CodeEditor';
import DebugMenu from './menus/DebugMenu/DebugMenu';
import useLocalStorage from '../hooks/useLocalStorage';
import { useRecoilValue } from 'recoil';
import { editorInteractionStateAtom } from '../atoms/editorInteractionStateAtom';
import BottomBar from './menus/BottomBar';
import QuadraticGrid from '../core/gridGL/QuadraticGrid';
import { useState } from 'react';
import { PixiApp } from '../core/gridGL/pixiApp/PixiApp';
import { SheetController } from '../core/transaction/sheetController';

interface Props {
  sheetController: SheetController;
}

export default function QuadraticUI(props: Props) {
  const [showDebugMenu] = useLocalStorage('showDebugMenu', false);
  const editorInteractionState = useRecoilValue(editorInteractionStateAtom);

  const [app] = useState(new PixiApp(props.sheetController.sheet));

  return (
    <div
      style={{
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
      }}
    >
      {editorInteractionState.showCellTypeMenu && <CellTypeMenu></CellTypeMenu>}
      {showDebugMenu && <DebugMenu sheet={props.sheetController.sheet} />}
      <TopBar app={app} sheet={props.sheetController.sheet} />

      <div
        style={{
          width: '100%',
          height: '100%',
          display: 'flex',
          overflow: 'hidden',
        }}
      >
        <QuadraticGrid sheetController={props.sheetController} app={app} />
        <CodeEditor editorInteractionState={editorInteractionState} sheet={props.sheetController.sheet} />
      </div>

      <BottomBar sheet={props.sheetController.sheet} />
    </div>
  );
}
