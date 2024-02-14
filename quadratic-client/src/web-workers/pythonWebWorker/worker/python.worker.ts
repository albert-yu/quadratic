import { PyodideInterface, loadPyodide } from 'pyodide';
import { PythonMessage } from '../pythonTypes';
const TRY_AGAIN_TIMEOUT = 500;

declare var self: any;

let getCellsMessages: (cells: { x: number; y: number; value: string }[]) => void | undefined;

const getCellsDB = async (
  x0: number,
  y0: number,
  x1: number,
  y1: number,
  sheet?: string,
  lineNumber?: number
): Promise<{ x: number; y: number; value: string }[]> => {
  return new Promise((resolve) => {
    getCellsMessages = (cells: { x: number; y: number; value: string }[]) => resolve(cells);
    self.postMessage({ type: 'get-cells', range: { x0, y0, x1, y1, sheet, lineNumber } } as PythonMessage);
  });
};

let pyodide: PyodideInterface | undefined;

async function pythonWebWorker() {
  try {
    self.postMessage({ type: 'not-loaded' } as PythonMessage);
    pyodide = await loadPyodide({ indexURL: '/pyodide' });

    pyodide.registerJsModule('getCellsDB', getCellsDB);
    await pyodide.loadPackage('micropip', { messageCallback: () => {} });

    const micropip = pyodide.pyimport('micropip');

    // patch requests https://github.com/koenvo/pyodide-http
    await micropip.install(['pyodide-http']);
    await pyodide.runPythonAsync('import pyodide_http; pyodide_http.patch_all();');

    // load our python code
    await micropip.install('/quadratic_py-0.1.0-py3-none-any.whl');

    // make run_python easier to call later
    await pyodide.runPython('from quadratic_py.run_python import run_python');
  } catch (e) {
    self.postMessage({ type: 'python-error' } as PythonMessage);
    console.warn(`[Python WebWorker] failed to load`, e);
    setTimeout(() => pythonWebWorker(), TRY_AGAIN_TIMEOUT);
    return;
  }

  self.postMessage({ type: 'python-loaded' } as PythonMessage);
}

self.onmessage = async (e: MessageEvent<PythonMessage>) => {
  const event = e.data;

  if (event.type === 'get-cells') {
    if (event.cells && getCellsMessages) {
      getCellsMessages(event.cells);
    }
  } else if (event.type === 'execute') {
    // make sure loading is done
    if (!pyodide) {
      self.postMessage({ type: 'not-loaded' } as PythonMessage);
    } else {
      const python_code = event.python;

      // auto load packages
      await pyodide.loadPackagesFromImports(python_code ?? '');

      const output = await pyodide.globals.get('run_python')(python_code);
      const results = Object.fromEntries(output.toJs());
      return self.postMessage({
        type: 'results',
        results,
        python_code,
      });
    }
  }
};

pythonWebWorker();
