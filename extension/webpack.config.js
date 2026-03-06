// ============================================================
// FILE:        webpack.config.js
// MODULE:      Layer 3 — Browser Extension > Build Configuration
// TASK:        T-031 (build tooling)
// PLATFORM:    chrome (MV3), firefox (MV2)
// AUTHOR:      FocusMe Co-Pilot (Claude Opus)
// GENERATED:   Phase 2, Session 3
// ============================================================

const path = require("path");
const CopyWebpackPlugin = require("copy-webpack-plugin");

const targetBrowser = process.env.TARGET_BROWSER || "chrome";
const isFirefox = targetBrowser === "firefox";
const manifestVersion = isFirefox ? "v2" : "v3";

module.exports = {
  entry: {
    background: "./src/background.ts",
    "content_scripts/element_blocker": "./src/content_scripts/element_blocker.ts",
    "popup/popup": "./src/popup/popup.ts",
    "blocked/blocked": "./src/blocked/blocked.ts",
  },
  output: {
    path: path.resolve(__dirname, `dist/${targetBrowser}`),
    filename: "[name].js",
    clean: true,
  },
  resolve: {
    extensions: [".ts", ".js"],
  },
  module: {
    rules: [
      {
        test: /\.ts$/,
        use: "ts-loader",
        exclude: /node_modules/,
      },
    ],
  },
  plugins: [
    new CopyWebpackPlugin({
      patterns: [
        // Copy the correct manifest for the target browser
        {
          from: `manifest.${manifestVersion}.json`,
          to: "manifest.json",
        },
        // Copy popup HTML
        { from: "src/popup/popup.html", to: "popup/popup.html" },
        { from: "src/popup/popup.css", to: "popup/popup.css" },
        // Copy blocked page
        { from: "src/blocked/blocked.html", to: "blocked/blocked.html" },
        { from: "src/blocked/blocked.css", to: "blocked/blocked.css" },
        // Copy icons
        { from: "icons", to: "icons", noErrorOnMissing: true },
      ],
    }),
  ],
  devtool: "source-map",
  optimization: {
    minimize: true,
  },
};
