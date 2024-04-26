import { events } from '@/app/events/events';
import { sheets } from '@/app/grid/controller/Sheets';
import { pixiApp } from '@/app/gridGL/pixiApp/PixiApp';
import { pixiAppSettings } from '@/app/gridGL/pixiApp/PixiAppSettings';
import { SheetPosTS } from '@/app/gridGL/types/size';
import { CURSOR_THICKNESS } from '@/app/gridGL/UI/Cursor';
import { convertColorStringToHex } from '@/app/helpers/convertColor';
import { focusGrid } from '@/app/helpers/focusGrid';
import { provideCompletionItems, provideHover } from '@/app/quadratic-rust-client/quadratic_rust_client';
import { FormulaLanguageConfig, FormulaTokenizerConfig } from '@/app/ui/menus/CodeEditor/FormulaLanguageModel';
import { quadraticCore } from '@/app/web-workers/quadraticCore/quadraticCore';
import { googleAnalyticsAvailable } from '@/shared/utils/analytics';
import mixpanel from 'mixpanel-browser';
import * as monaco from 'monaco-editor';
import { editor } from 'monaco-editor';
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
import { Rectangle } from 'pixi.js';

window.MonacoEnvironment = {
  getWorker(_, label) {
    return new editorWorker();
  },
};

// Pixels needed when growing width to avoid monaco from scrolling the text
// (determined by experimentation).
const PADDING_FOR_GROWING_HORIZONTALLY = 20;

// Pixels needed when for minWidth to avoid monaco from scrolling the text
// (determined by experimentation).
const PADDING_FOR_MIN_WIDTH = 30;

// Minimum amount to scroll viewport when cursor is near the edge.
const MINIMUM_MOVE_VIEWPORT = 50;

const theme: editor.IStandaloneThemeData = {
  base: 'vs',
  inherit: true,
  rules: [],
  colors: {},
};
monaco.editor.defineTheme('inline-editor', theme);

class InlineEditorHandler {
  private div?: HTMLDivElement;
  private editor?: editor.IStandaloneCodeEditor;
  private location?: SheetPosTS;
  private cellOffsets?: Rectangle;
  private width = 0;
  private height = 0;
  private open = false;

  private temporaryBold = false;
  private temporaryItalic = false;

  private formula = false;

  // this is used to calculate the width of the editor
  private sizingDiv: HTMLDivElement;

  // this is used to display the formula expand button
  private formulaExpandDiv?: HTMLDivElement;

  constructor() {
    events.on('changeInput', this.changeInput);
    this.sizingDiv = document.createElement('div');
    this.sizingDiv.style.visibility = 'hidden';
    this.sizingDiv.style.width = 'fit-content';
    this.sizingDiv.style.fontSize = '14px';
    this.sizingDiv.style.fontFamily = 'OpenSans';
    this.sizingDiv.style.paddingLeft = CURSOR_THICKNESS + 'px';
    this.sizingDiv.style.whiteSpace = 'nowrap';
  }

  private reset() {
    this.formula = false;
    this.temporaryBold = false;
    this.temporaryItalic = false;
    this.width = 0;
    this.height = 0;
    this.open = false;
  }

  // viewport should try to keep the cursor in view
  private keepCursorVisible = () => {
    if (!this.editor) return;
    const editorPosition = this.editor.getPosition();
    if (!editorPosition) return;
    const position = this.editor.getScrolledVisiblePosition(editorPosition);
    if (!position) return;
    const domNode = this.editor.getDomNode();
    if (!domNode) return;
    const bounds = domNode.getBoundingClientRect();
    const canvas = pixiApp.canvas.getBoundingClientRect();
    const cursor = position.left + bounds.left;
    const worldCursorTop = pixiApp.viewport.toWorld(cursor, bounds.top - canvas.top);
    const worldCursorBottom = pixiApp.viewport.toWorld(cursor, bounds.bottom - canvas.top);
    const viewportBounds = pixiApp.viewport.getVisibleBounds();
    let x = 0,
      y = 0;
    if (worldCursorTop.x > viewportBounds.right) {
      x = viewportBounds.right - (worldCursorTop.x + MINIMUM_MOVE_VIEWPORT);
    } else if (worldCursorTop.x < viewportBounds.left) {
      x = viewportBounds.left + MINIMUM_MOVE_VIEWPORT - worldCursorTop.x;
    }
    if (worldCursorBottom.y > viewportBounds.bottom) {
      y = viewportBounds.bottom - (worldCursorBottom.y + MINIMUM_MOVE_VIEWPORT);
    } else if (worldCursorTop.y < viewportBounds.top) {
      y = viewportBounds.top + MINIMUM_MOVE_VIEWPORT - worldCursorTop.y;
    }
    if (x || y) {
      pixiApp.viewport.x += x;
      pixiApp.viewport.y += y;
      pixiApp.setViewportDirty();
    }
  };

  private changeInput = async (input: boolean, initialValue?: string) => {
    if (!this.div || !this.editor) {
      throw new Error('Expected div and editor to be defined in InlineEditorHandler');
    }

    if (input) {
      this.open = true;
      this.div.style.display = 'flex';
      const sheet = sheets.sheet;
      this.location = {
        sheetId: sheet.id,
        x: sheet.cursor.originPosition.x,
        y: sheet.cursor.originPosition.y,
      };
      let value: string;
      if (initialValue) {
        value = initialValue;
      } else {
        const formula = await quadraticCore.getCodeCell(sheets.sheet.id, this.location.x, this.location.y);
        if (formula?.language === 'Formula') {
          this.formula = true;
          value = '=' + formula.code_string;
        } else {
          value = (await quadraticCore.getEditCell(sheets.sheet.id, this.location.x, this.location.y)) || '';
        }
      }
      this.editor.setValue(value);
      if (initialValue) {
        this.editor.setPosition({ lineNumber: 1, column: initialValue.length + 1 });
      }
      const format = await quadraticCore.getCellFormatSummary(sheets.sheet.id, this.location.x, this.location.y);
      theme.colors['editor.background'] = format.fillColor ? convertColorStringToHex(format.fillColor) : '#ffffff';
      monaco.editor.defineTheme('inline-editor', theme);
      this.cellOffsets = sheet.getCellOffsets(this.location.x, this.location.y);
      this.div.style.left = this.cellOffsets.x + CURSOR_THICKNESS + 'px';
      this.div.style.top = this.cellOffsets.y + 2 + 'px';
      this.height = this.cellOffsets.height - 4;
      if (!this.formulaExpandDiv) {
        throw new Error('Expected formulaExpandDiv to be defined in InlineEditorHandler');
      }
      this.formulaExpandDiv.style.height = this.height + 'px';
      this.formulaExpandDiv.style.lineHeight = this.height + 'px';
      this.updateCursorPosition();
      this.keepCursorVisible();
      this.editor.focus();
    } else {
      this.div.style.display = 'none';
      this.reset();
    }
  };

  private updateCursorPosition = () => {
    // this will get called upon opening (before variables are set), and after every cursor movement
    if (!this.div || !this.editor || !this.cellOffsets) return;

    const value = this.editor.getValue();
    if (value[0] === '=') {
      this.changeToFormula(true);
    } else if (this.formula && value[0] !== '=') {
      this.changeToFormula(false);
    }
    this.sizingDiv.innerHTML = ' ' + value;
    this.width =
      Math.max(this.cellOffsets.width - PADDING_FOR_MIN_WIDTH, this.sizingDiv.offsetWidth) +
      PADDING_FOR_GROWING_HORIZONTALLY;
    this.editor.layout({ width: this.width, height: this.height });
    pixiApp.cursor.dirty = true;
  };

  private changeToFormula = (formula: boolean) => {
    if (this.formula === formula) return;
    if (!this.editor) {
      throw new Error('Expected editor to be defined in InlineEditorHandler');
    }
    if (!this.formulaExpandDiv) {
      throw new Error('Expected formulaExpandDiv to be defined in InlineEditorHandler');
    }
    this.formula = formula;
    const model = this.editor.getModel();
    if (!model) {
      throw new Error('Expected model to be defined in changeToFormula');
    }
    if (formula) {
      editor.setModelLanguage(model, 'Formula');

      // need to show the change to A1 notation
      pixiApp.headings.dirty = true;
    } else {
      editor.setModelLanguage(model, 'plaintext');
    }
    this.formulaExpandDiv.style.display = formula ? 'block' : 'none';
  };

  private keyDown = (e: monaco.IKeyboardEvent) => {
    if (e.code === 'Escape') {
      this.close(0, 0, true);
      e.stopPropagation();
    } else if (e.code === 'Enter') {
      this.close(0, 1, false);
      e.stopPropagation();
    }
  };

  private close = (deltaX = 0, deltaY = 0, cancel: boolean) => {
    if (!this.editor || !this.location) {
      throw new Error('Expected editor and location to be defined in InlineEditorHandler');
    }

    const value = this.editor.getValue();

    if (!cancel && value.trim()) {
      if (this.formula) {
        quadraticCore.setCodeCellValue({
          sheetId: sheets.sheet.id,
          x: this.location.x,
          y: this.location.y,
          language: 'Formula',
          codeString: value.slice(1),
          cursor: sheets.getCursorPosition(),
        });
        mixpanel.track('[CodeEditor].cellRun', {
          type: 'Formula',
          inline: true,
        });
        // Google Ads Conversion for running a cell
        if (googleAnalyticsAvailable()) {
          //@ts-expect-error
          gtag('event', 'conversion', {
            send_to: 'AW-11007319783/C-yfCJOe6JkZEOe92YAp',
          });
        }
      } else {
        quadraticCore.setCellValue(
          sheets.sheet.id,
          this.location.x,
          this.location.y,
          value,
          sheets.getCursorPosition()
        );
      }
    }

    // Update Grid Interaction state, reset input value state
    const position = sheets.sheet.cursor.cursorPosition;
    sheets.sheet.cursor.changePosition({
      cursorPosition: {
        x: position.x + deltaX,
        y: position.y + deltaY,
      },
    });

    this.reset();
    pixiAppSettings.changeInput(false);

    // Set focus back to Grid
    focusGrid();
  };

  private openCodeEditor = (e: MouseEvent) => {
    e.stopPropagation();

    // todo: open in code editor
  };

  attach(div: HTMLDivElement) {
    if (this.div) throw new Error('Inline editor already attached');
    div.style.display = 'none';
    this.div = div;

    monaco.languages.register({ id: 'Formula' });
    monaco.languages.setLanguageConfiguration('Formula', FormulaLanguageConfig);
    monaco.languages.setMonarchTokensProvider('Formula', FormulaTokenizerConfig);
    monaco.languages.registerCompletionItemProvider('Formula', {
      provideCompletionItems,
    });
    monaco.languages.registerHoverProvider('Formula', { provideHover });

    this.editor = editor.create(div.childNodes[0] as HTMLDivElement, {
      automaticLayout: false,
      readOnly: false,
      renderLineHighlight: 'none',
      // quickSuggestions: false,
      glyphMargin: false,
      lineDecorationsWidth: 0,
      folding: false,
      fixedOverflowWidgets: false,
      revealHorizontalRightPadding: 0,
      disableMonospaceOptimizations: true,
      roundedSelection: false,
      contextmenu: false,
      links: false,
      minimap: { enabled: false },
      overviewRulerLanes: 0,
      cursorWidth: CURSOR_THICKNESS,
      padding: { top: 0, bottom: 0 },
      hideCursorInOverviewRuler: true,
      overviewRulerBorder: false,
      wordWrap: 'off',
      occurrencesHighlight: false,
      wordBasedSuggestions: false,
      find: {
        addExtraSpaceOnTop: false,
        autoFindInSelection: 'never',
        seedSearchStringFromSelection: 'never',
      },
      fontSize: 14,
      lineHeight: 15,
      fontFamily: 'OpenSans',
      fontWeight: 'normal',
      lineNumbers: 'off',
      lineNumbersMinChars: 0,
      scrollBeyondLastColumn: 0,
      scrollBeyondLastLine: false,
      scrollbar: {
        horizontal: 'hidden',
        vertical: 'hidden',
        alwaysConsumeMouseWheel: false,
      },
      theme: 'inline-editor',
      stickyScroll: { enabled: false },
      language: this.formula ? 'formula' : undefined,
    });

    document.body.appendChild(this.sizingDiv);
    this.formulaExpandDiv = div.childNodes[1]! as HTMLDivElement;
    this.formulaExpandDiv.addEventListener('click', this.openCodeEditor);

    this.editor.onDidChangeCursorPosition(this.updateCursorPosition);
    this.editor.onKeyDown(this.keyDown);
    this.editor.onDidChangeCursorPosition(this.keepCursorVisible);
  }

  isEditingFormula() {
    return this.open && this.formula;
  }
}

export const inlineEditorHandler = new InlineEditorHandler();
