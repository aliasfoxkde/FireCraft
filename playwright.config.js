const { defineConfig } = require('@playwright/test');
const use = {
  timeout: 30000,
};
if (process.env.FIRECRAFT_CHROMIUM_PATH) {
  use.launchOptions = {
    executablePath: process.env.FIRECRAFT_CHROMIUM_PATH,
    args: ['--no-sandbox', '--enable-unsafe-swiftshader'],
  };
}
module.exports = defineConfig({
  testDir: './web',
  use,
  forbidOnly: false,
  retries: 0,
});
