import {startModuleApp} from './shared/module-runtime.mjs';

const events = Object.freeze({
  startup_started: 'info',
  startup_ready: 'info',
  startup_failed: 'error',
  runtime_failed: 'error',
  geometry_updated: 'info',
  geometry_rejected: 'warn',
  geometry_worker_disconnected: 'error',
});

let ready = false;
let failed = false;
window.mathDiagnostic = (event, segments) => {
  if (!Object.hasOwn(events, event)) return;
  const level = events[event];
  try {
    const record = {timestamp: new Date().toISOString(), level, event, platform: 'browser'};
    if (event === 'geometry_updated' && Number.isSafeInteger(segments) && segments >= 0) {
      record.segments = segments;
    }
    console[level](record);
  } catch {
    // Diagnostic output must never interfere with the app or status UI.
  }
};

window.mathDiagnostic('startup_started');
const runtime = startModuleApp({start: async () => {
  const {default: init} = await import('./math_playground.js');
  await init();
}}, {
  document,
  addEventListener: window.addEventListener.bind(window),
  console: {error() {
    if (failed) return;
    failed = true;
    window.mathDiagnostic(ready ? 'runtime_failed' : 'startup_failed');
  }},
});
window.appReady = message => {
  runtime.ready(message);
  if (!failed && !ready) {
    ready = true;
    window.mathDiagnostic('startup_ready');
  }
};
window.appError = message => {
  runtime.fail(message);
  if (!failed) {
    failed = true;
    window.mathDiagnostic(ready ? 'runtime_failed' : 'startup_failed');
  }
};
