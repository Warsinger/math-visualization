import {startModuleApp} from './shared/module-runtime.mjs';
startModuleApp({start: async () => {
  const {default: init} = await import('./math_playground.js');
  await init();
}});
