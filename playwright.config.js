const { defineConfig } = require('@playwright/test');
module.exports = defineConfig({
  testDir: './web',
  use: {
    timeout: 30000,
  },
  forbidOnly: false,
  retries: 0,
});
