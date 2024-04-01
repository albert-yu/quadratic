import { sheets } from '@/grid/controller/Sheets';
import { Coordinate, SheetPosTS } from '@/gridGL/types/size';
import { multiplayer } from '@/web-workers/multiplayerWebWorker/multiplayer';
import { EvaluationResult } from '@/web-workers/pythonWebWorker/pythonTypes';
import { quadraticCore } from '@/web-workers/quadraticCore/quadraticCore';
import mixpanel from 'mixpanel-browser';
import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useRecoilState } from 'recoil';
// TODO(ddimaria): leave this as we're looking to add this back in once improved
// import { Diagnostic } from 'vscode-languageserver-types';
import { usePythonState } from '@/atoms/usePythonState';
import { events } from '@/events/events';
import { JsCodeCell, Pos } from '@/quadratic-core-types';
import { googleAnalyticsAvailable } from '@/utils/analytics';
import { hasPermissionToEditFile } from '../../../actions';
import { editorInteractionStateAtom } from '../../../atoms/editorInteractionStateAtom';
import { pixiApp } from '../../../gridGL/pixiApp/PixiApp';
import { focusGrid } from '../../../helpers/focusGrid';
import { pythonWebWorker } from '../../../web-workers/pythonWebWorker/pythonWebWorker';
import './CodeEditor.css';
import { CodeEditorBody } from './CodeEditorBody';
import { CodeEditorProvider } from './CodeEditorContext';
import { CodeEditorHeader } from './CodeEditorHeader';
import { Console } from './Console';
import { ResizeControl } from './ResizeControl';
import { ReturnTypeInspector } from './ReturnTypeInspector';
import { SaveChangesAlert } from './SaveChangesAlert';

export const CodeEditor = () => {
  const [editorInteractionState, setEditorInteractionState] = useRecoilState(editorInteractionStateAtom);
  const { showCodeEditor, mode: editorMode } = editorInteractionState;

  const { pythonState } = usePythonState();

  // update code cell
  const [codeString, setCodeString] = useState('');

  // code info
  const [out, setOut] = useState<{ stdOut?: string; stdErr?: string } | undefined>(undefined);
  const [evaluationResult, setEvaluationResult] = useState<EvaluationResult | undefined>(undefined);
  const [spillError, setSpillError] = useState<Coordinate[] | undefined>();

  const [editorWidth, setEditorWidth] = useState<number>(
    window.innerWidth * 0.35 // default to 35% of the window width
  );
  const [consoleHeight, setConsoleHeight] = useState<number>(200);
  const [showSaveChangesAlert, setShowSaveChangesAlert] = useState(false);
  const [editorContent, setEditorContent] = useState<string | undefined>(codeString);
  // TODO(ddimaria): leave this as we're looking to add this back in once improved
  // const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([]);

  const cellLocation: SheetPosTS = useMemo(() => {
    return {
      x: editorInteractionState.selectedCell.x,
      y: editorInteractionState.selectedCell.y,
      sheetId: editorInteractionState.selectedCellSheet,
    };
  }, [
    editorInteractionState.selectedCell.x,
    editorInteractionState.selectedCell.y,
    editorInteractionState.selectedCellSheet,
  ]);

  const unsaved = useMemo(() => {
    return editorContent !== codeString;
  }, [codeString, editorContent]);

  // handle someone trying to open a different code editor
  useEffect(() => {
    if (editorInteractionState.waitingForEditorClose) {
      // if unsaved then show save dialog and wait for that to complete
      if (unsaved) {
        setShowSaveChangesAlert(true);
      }

      // otherwise either open the new editor or show the cell type menu (if type is not selected)
      else {
        const waitingForEditorClose = editorInteractionState.waitingForEditorClose;
        if (waitingForEditorClose) {
          setEditorInteractionState((oldState) => ({
            ...oldState,
            selectedCell: waitingForEditorClose.selectedCell,
            selectedCellSheet: waitingForEditorClose.selectedCellSheet,
            mode: waitingForEditorClose.mode,
            showCodeEditor: !waitingForEditorClose.showCellTypeMenu,
            showCellTypeMenu: waitingForEditorClose.showCellTypeMenu,
            waitingForEditorClose: undefined,
          }));
        }
      }
    }
  }, [editorInteractionState.waitingForEditorClose, setEditorInteractionState, unsaved]);

  const updateCodeCell = useCallback(
    async (pushCodeCell?: JsCodeCell) => {
      // selectedCellSheet may be undefined if code editor was activated from within the CellInput
      if (!editorInteractionState.selectedCellSheet) return;
      const codeCell =
        pushCodeCell ??
        (await quadraticCore.getCodeCell(
          editorInteractionState.selectedCellSheet,
          editorInteractionState.selectedCell.x,
          editorInteractionState.selectedCell.y
        ));

      if (codeCell) {
        setCodeString(codeCell.code_string);
        setOut({ stdOut: codeCell.std_out ?? undefined, stdErr: codeCell.std_err ?? undefined });

        if (!pushCodeCell) setEditorContent(codeCell.code_string);
        const evaluationResult = codeCell.evaluation_result ? JSON.parse(codeCell.evaluation_result) : {};
        setEvaluationResult({ ...evaluationResult, ...codeCell.return_info });
        setSpillError(codeCell.spill_error?.map((c: Pos) => ({ x: Number(c.x), y: Number(c.y) } as Coordinate)));
      } else {
        setCodeString('');
        if (!pushCodeCell) setEditorContent('');
        setEvaluationResult(undefined);
        setOut(undefined);
      }
    },
    [
      editorInteractionState.selectedCell.x,
      editorInteractionState.selectedCell.y,
      editorInteractionState.selectedCellSheet,
    ]
  );

  // ensure codeCell is created w/content and updated when it receives a change request from Rust
  useEffect(() => {
    updateCodeCell();

    const update = (options: { sheetId: string; x: number; y: number; codeCell?: JsCodeCell }) => {
      if (options.sheetId === cellLocation.sheetId || options.x === cellLocation.x || options.y === cellLocation.y) {
        updateCodeCell(options.codeCell);
      }
    };
    events.on('updateCodeCell', update);
    return () => {
      events.off('updateCodeCell', update);
    };
  }, [cellLocation.sheetId, cellLocation.x, cellLocation.y, updateCodeCell]);

  // TODO(ddimaria): leave this as we're looking to add this back in once improved
  // useEffect(() => {
  //   const updateDiagnostics = (e: Event) => setDiagnostics((e as CustomEvent).detail.diagnostics);
  //   window.addEventListener('python-diagnostics', updateDiagnostics);
  //   return () => {
  //     window.removeEventListener('python-diagnostics', updateDiagnostics);
  //   };
  // }, [updateCodeCell]);

  useEffect(() => {
    mixpanel.track('[CodeEditor].opened', { type: editorMode });
    multiplayer.sendCellEdit('', 0, true);
  }, [editorMode]);

  const closeEditor = useCallback(
    (skipSaveCheck: boolean) => {
      if (!skipSaveCheck && unsaved) {
        setShowSaveChangesAlert(true);
      } else {
        setEditorInteractionState((oldState) => ({
          ...oldState,
          editorEscapePressed: false,
          showCodeEditor: false,
        }));
        pixiApp.highlightedCells.clear();
        focusGrid();
        multiplayer.sendEndCellEdit();
      }
    },
    [setEditorInteractionState, unsaved]
  );

  // handle when escape is pressed when escape does not have focus
  useEffect(() => {
    if (editorInteractionState.editorEscapePressed) {
      if (unsaved) {
        setShowSaveChangesAlert(true);
      } else {
        closeEditor(true);
      }
    }
  }, [closeEditor, editorInteractionState.editorEscapePressed, unsaved]);

  const saveAndRunCell = async () => {
    const language = editorInteractionState.mode;

    if (language === undefined)
      throw new Error(`Language ${editorInteractionState.mode} not supported in CodeEditor#saveAndRunCell`);
    quadraticCore.setCodeCellValue({
      sheetId: cellLocation.sheetId,
      x: cellLocation.x,
      y: cellLocation.y,
      codeString: editorContent ?? '',
      language,
      cursor: sheets.getCursorPosition(),
    });

    setCodeString(editorContent ?? '');

    mixpanel.track('[CodeEditor].cellRun', {
      type: editorMode,
    });
    // Google Ads Conversion for running a cell
    if (googleAnalyticsAvailable()) {
      //@ts-expect-error
      gtag('event', 'conversion', {
        send_to: 'AW-11007319783/C-yfCJOe6JkZEOe92YAp',
      });
    }
  };

  const cancelPython = () => {
    if (pythonState !== 'running') return;

    pythonWebWorker.cancelExecution();
  };

  const onKeyDownEditor = (event: React.KeyboardEvent<HTMLDivElement>) => {
    // Don't allow the shortcuts below for certain users
    if (!hasPermissionToEditFile(editorInteractionState.permissions)) {
      return;
    }

    // Command + S
    if ((event.metaKey || event.ctrlKey) && event.key === 's') {
      event.preventDefault();
      saveAndRunCell();
    }

    // Command + Enter
    if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') {
      event.preventDefault();
      event.stopPropagation();
      saveAndRunCell();
    }

    // Command + Escape
    if ((event.metaKey || event.ctrlKey) && event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      cancelPython();
    }
  };

  const afterDialog = () => {
    setShowSaveChangesAlert(false);
    if (editorInteractionState.editorEscapePressed) {
      closeEditor(true);
    }
    const waitingForEditorClose = editorInteractionState.waitingForEditorClose;
    if (waitingForEditorClose) {
      setEditorInteractionState((oldState) => ({
        ...oldState,
        selectedCell: waitingForEditorClose.selectedCell,
        selectedCellSheet: waitingForEditorClose.selectedCellSheet,
        mode: waitingForEditorClose.mode,
        showCodeEditor: !waitingForEditorClose.showCellTypeMenu,
        showCellTypeMenu: waitingForEditorClose.showCellTypeMenu,
        waitingForEditorClose: undefined,
      }));
    } else {
      closeEditor(true);
    }
  };

  if (!showCodeEditor) {
    return null;
  }

  return (
    <CodeEditorProvider>
      <div
        id="QuadraticCodeEditorID"
        style={{
          position: 'absolute',
          top: 0,
          right: 0,
          bottom: 0,
          display: 'flex',
          flexDirection: 'column',
          width: `${editorWidth}px`,
          minWidth: '350px',
          maxWidth: '90%',
          backgroundColor: '#ffffff',
          zIndex: 2,
        }}
        onKeyDownCapture={onKeyDownEditor}
        onPointerEnter={() => {
          // todo: handle multiplayer code editor here
          multiplayer.sendMouseMove();
        }}
        onPointerMove={(e) => {
          e.stopPropagation();
        }}
      >
        {showSaveChangesAlert && (
          <SaveChangesAlert
            onCancel={() => {
              setShowSaveChangesAlert(!showSaveChangesAlert);
              setEditorInteractionState((old) => ({
                ...old,
                editorEscapePressed: false,
                waitingForEditorClose: undefined,
              }));
            }}
            onSave={() => {
              saveAndRunCell();
              afterDialog();
            }}
            onDiscard={() => {
              afterDialog();
            }}
          />
        )}

        <ResizeControl setState={setEditorWidth} position="LEFT" />
        <CodeEditorHeader
          cellLocation={cellLocation}
          unsaved={unsaved}
          saveAndRunCell={saveAndRunCell}
          cancelPython={cancelPython}
          closeEditor={() => closeEditor(false)}
        />
        <CodeEditorBody
          editorContent={editorContent}
          setEditorContent={setEditorContent}
          closeEditor={closeEditor}
          evaluationResult={evaluationResult}
        />
        {editorInteractionState.mode === 'Python' && (
          <ReturnTypeInspector
            evaluationResult={evaluationResult}
            show={Boolean(evaluationResult?.line_number && !out?.stdErr && !unsaved)}
          />
        )}

        <ResizeControl setState={setConsoleHeight} position="TOP" />
        {/* Console Wrapper */}
        <div
          style={{
            position: 'relative',
            display: 'flex',
            flexDirection: 'column',
            minHeight: '100px',
            background: '#fff',
            height: `${consoleHeight}px`,
          }}
        >
          {(editorInteractionState.mode === 'Python' || editorInteractionState.mode === 'Formula') && (
            <Console
              consoleOutput={out}
              editorMode={editorMode}
              editorContent={editorContent}
              evaluationResult={evaluationResult}
              spillError={spillError}
            />
          )}
        </div>
      </div>
    </CodeEditorProvider>
  );
};
