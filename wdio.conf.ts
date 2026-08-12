import type { Options } from "@wdio/types";

export const config: Options.Testrunner = {
  runner: "local",
  specs: ["./e2e/**/*.e2e.ts"],
  maxInstances: 1,
  capabilities: [{
    browserName: "tauri",
    timeouts: { script: 300_000 },
    "tauri:options": {
      application: "./src-tauri/target/debug/bkuw",
    },
  }],
  services: [["tauri", { driverProvider: "embedded", autoInstallTauriDriver: false }]],
  framework: "mocha",
  reporters: ["spec"],
  logLevel: "warn",
  waitforTimeout: 10_000,
  connectionRetryTimeout: 30_000,
  mochaOpts: { ui: "bdd", timeout: 300_000 },
};
